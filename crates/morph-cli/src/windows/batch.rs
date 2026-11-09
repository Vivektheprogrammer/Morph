use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type HANDLE = isize;
const WAIT_OBJECT_0: u32 = 0x00000000;
const WAIT_ABANDONED: u32 = 0x00000080;
const WAIT_TIMEOUT: u32 = 0x00000102;

#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *const std::ffi::c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> HANDLE;
    fn ReleaseMutex(hMutex: HANDLE) -> i32;
    fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: u32) -> u32;
    fn CloseHandle(hObject: HANDLE) -> i32;
}

#[link(name = "shlwapi")]
extern "system" {
    fn StrCmpLogicalW(psz1: *const u16, psz2: *const u16) -> i32;
}

/// Natural sort comparison using Windows Shell StrCmpLogicalW API.
/// This matches Windows File Explorer's exact visual item order (e.g. "A" < "B", "file2" < "file10").
pub fn natural_cmp(a: &Path, b: &Path) -> std::cmp::Ordering {
    let name_a = a.file_name().unwrap_or_default().to_string_lossy();
    let name_b = b.file_name().unwrap_or_default().to_string_lossy();

    let wide_a: Vec<u16> = name_a.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_b: Vec<u16> = name_b.encode_utf16().chain(std::iter::once(0)).collect();

    let res = unsafe { StrCmpLogicalW(wide_a.as_ptr(), wide_b.as_ptr()) };
    if res < 0 {
        std::cmp::Ordering::Less
    } else if res > 0 {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

/// Ticket entry representing a file in the multi-select batch.
struct TicketEntry {
    file_path: PathBuf,
    ticket_file: PathBuf,
}

/// Coordinates a batch of concurrent process invocations triggered by Windows Explorer.
///
/// When multiple files are selected in Explorer and a context menu verb is clicked,
/// Explorer spawns an independent process for each selected file in rapid succession.
/// Explorer always invokes whichever file was right-clicked / focused FIRST, followed by
/// the remaining selected items.
///
/// This coordinator elects a Leader process and gracefully finishes Sibling processes:
/// - Each process writes a microsecond/nanosecond ticket to `%TEMP%\morph_batch_<hash>\`.
/// - Each process attempts to acquire a named Windows Mutex `Local\Morph_Batch_<hash>`.
/// - The **Leader** acquires the mutex, enters an active debounce window (120ms idle timeout,
///   350ms cap) to allow all sibling processes spawned by Explorer to write their tickets.
/// - Once the batch settles, the Leader reads all tickets, cleans up the ticket files,
///   sorts all collected files in Windows File Explorer's natural visual display order
///   (`StrCmpLogicalW`), releases the mutex, and returns `Some(Vec<PathBuf>)`.
/// - Each **Sibling** detects that the mutex is already held, recognizes that its ticket
///   is safely queued for the Leader, and returns `None` so it can exit code 0 immediately.
pub fn coordinate_batch(initial_path: &Path, action_tag: &str) -> Option<Vec<PathBuf>> {
    // 1. Determine scoped batch directory and named mutex key based on parent folder + action
    let parent_dir = initial_path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_else(|| {
            initial_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        });

    let mut hasher = DefaultHasher::new();
    parent_dir.to_string_lossy().to_lowercase().hash(&mut hasher);
    action_tag.hash(&mut hasher);
    let batch_hash = format!("{:016x}", hasher.finish());

    let batch_dir = std::env::temp_dir().join(format!("morph_batch_{}", batch_hash));
    if let Err(_) = fs::create_dir_all(&batch_dir) {
        // Fallback: if temp dir creation fails, execute single file
        return Some(vec![initial_path.to_path_buf()]);
    }

    // 2. Write high-resolution ticket with arrival timestamp and PID
    let arrival_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let ticket_filename = format!("{:020}_{}.ticket", arrival_nanos, pid);
    let ticket_path = batch_dir.join(&ticket_filename);

    let path_str = initial_path.to_string_lossy();
    if let Err(_) = fs::write(&ticket_path, path_str.as_bytes()) {
        // Fallback: if ticket write fails, execute single file
        return Some(vec![initial_path.to_path_buf()]);
    }

    // 3. Create or open the Windows Named Mutex
    let mutex_name = format!("Local\\Morph_Batch_{}\0", batch_hash);
    let wide_name: Vec<u16> = mutex_name.encode_utf16().collect();

    let mutex_handle: HANDLE =
        unsafe { CreateMutexW(std::ptr::null(), 0, wide_name.as_ptr()) };

    if mutex_handle == 0 || mutex_handle == -1 {
        // Failed to create mutex, fallback to single file
        let _ = fs::remove_file(&ticket_path);
        return Some(vec![initial_path.to_path_buf()]);
    }

    // 4. Try to acquire the mutex (wait up to 35ms)
    // Sibling processes spawned by Explorer will find the mutex held by the leader.
    let wait_res = unsafe { WaitForSingleObject(mutex_handle, 35) };

    if wait_res == WAIT_TIMEOUT {
        // Mutex is held by the Leader.
        // Our ticket is written and queued in `batch_dir`. The Leader will consume it.
        unsafe { CloseHandle(mutex_handle) };
        return None;
    }

    if wait_res != WAIT_OBJECT_0 && wait_res != WAIT_ABANDONED {
        // Some error occurred acquiring the mutex. Fall back to processing single file.
        let _ = fs::remove_file(&ticket_path);
        unsafe { CloseHandle(mutex_handle) };
        return Some(vec![initial_path.to_path_buf()]);
    }

    // 5. WE ARE THE LEADER!
    // Enter debounce loop to allow all sibling processes spawned by Explorer to write their tickets.
    let start_wait = Instant::now();
    let mut last_ticket_count = 0;
    let mut last_change = Instant::now();
    let debounce_window = Duration::from_millis(120);
    let max_wait = Duration::from_millis(350);

    loop {
        std::thread::sleep(Duration::from_millis(25));

        let current_count = count_tickets(&batch_dir);
        if current_count != last_ticket_count {
            last_ticket_count = current_count;
            last_change = Instant::now();
        }

        if last_change.elapsed() >= debounce_window || start_wait.elapsed() >= max_wait {
            break;
        }
    }

    // 6. Collect all tickets
    let mut tickets = Vec::new();

    if let Ok(entries) = fs::read_dir(&batch_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ticket") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        let target_path = PathBuf::from(trimmed);
                        tickets.push(TicketEntry {
                            file_path: target_path,
                            ticket_file: path,
                        });
                    }
                }
            }
        }
    }

    // Clean up ticket files
    for t in &tickets {
        let _ = fs::remove_file(&t.ticket_file);
    }

    // 7. Release mutex and clean up directory
    unsafe {
        ReleaseMutex(mutex_handle);
        CloseHandle(mutex_handle);
    }
    let _ = fs::remove_dir(&batch_dir);

    // Deduplicate while preserving files
    let mut seen = std::collections::HashSet::new();
    let mut collected = Vec::new();
    for t in tickets {
        let key = t
            .file_path
            .canonicalize()
            .unwrap_or_else(|_| t.file_path.clone());
        if seen.insert(key) {
            collected.push(t.file_path);
        }
    }

    // Sort collected files strictly in Windows File Explorer natural display order (A before B, 1 before 2)
    collected.sort_by(|a, b| natural_cmp(a, b));

    if collected.is_empty() {
        Some(vec![initial_path.to_path_buf()])
    } else {
        Some(collected)
    }
}

fn count_tickets(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("ticket") {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort_ordering() {
        let file_a = Path::new(r"C:\test\A.png");
        let file_b = Path::new(r"C:\test\B.png");
        assert_eq!(natural_cmp(file_a, file_b), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp(file_b, file_a), std::cmp::Ordering::Greater);

        let file_2 = Path::new(r"C:\test\img2.jpg");
        let file_10 = Path::new(r"C:\test\img10.jpg");
        assert_eq!(natural_cmp(file_2, file_10), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_coordinate_batch_single() {
        let temp = std::env::temp_dir().join("morph_test_single");
        let _ = fs::create_dir_all(&temp);
        let file1 = temp.join("document1.png");
        let _ = fs::write(&file1, b"sample");

        let result = coordinate_batch(&file1, "test_single");
        assert!(result.is_some());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file1);
    }

    #[test]
    fn test_coordinate_batch_concurrent() {
        let temp = std::env::temp_dir().join("morph_test_concurrent");
        let _ = fs::create_dir_all(&temp);
        let file_a = temp.join("item_a.png");
        let file_b = temp.join("item_b.jpg");
        let _ = fs::write(&file_a, b"a");
        let _ = fs::write(&file_b, b"b");

        // Spawn B first (simulating user right-clicking on B)
        let fb_clone = file_b.clone();
        let handle_b = std::thread::spawn(move || {
            coordinate_batch(&fb_clone, "test_concurrent")
        });

        // Sibling A arrives 25ms later
        std::thread::sleep(Duration::from_millis(25));
        let fa_clone = file_a.clone();
        let handle_a = std::thread::spawn(move || {
            coordinate_batch(&fa_clone, "test_concurrent")
        });

        let res_b = handle_b.join().unwrap();
        let res_a = handle_a.join().unwrap();

        // One of them is the leader (Some), one is the sibling (None)
        let leader_res = res_b.or(res_a).expect("At least one process must be elected leader");
        assert_eq!(leader_res.len(), 2);
        // Even though B was launched first by right-click, natural sort puts A first and B second!
        assert_eq!(leader_res[0], file_a);
        assert_eq!(leader_res[1], file_b);
    }
}
