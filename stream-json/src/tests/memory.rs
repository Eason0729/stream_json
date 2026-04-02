use std::fs;
use std::hint::black_box;

pub(super) fn memory_usage() -> Option<MemoryUsage> {
    if let Ok(smaps) = fs::read_to_string("/proc/self/smaps") {
        let mut total_size_kb = 0usize;
        let mut total_rss_kb = 0usize;

        for line in smaps.lines() {
            if let Some(rest) = line.strip_prefix("Size:") {
                total_size_kb += scan_int(rest).0;
            } else if let Some(rest) = line.strip_prefix("Rss:") {
                total_rss_kb += scan_int(rest).0;
            }
        }

        return Some(MemoryUsage {
            physical_mem: total_rss_kb << 10,
            virtual_mem: total_size_kb << 10,
        });
    }

    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let page_size = page_size()?;
    let (virtual_pages, idx) = scan_int(&statm);
    let (physical_pages, _) = scan_int(&statm[idx..]);

    Some(MemoryUsage {
        physical_mem: physical_pages * page_size,
        virtual_mem: virtual_pages * page_size,
    })
}

pub(super) fn assert_physical_memory_increases_by_at_least<F, T>(min_delta: usize, f: F)
where
    F: FnOnce() -> T,
{
    let before = memory_usage().expect("failed to read memory usage before allocation");
    let value = f();
    let after = memory_usage().expect("failed to read memory usage after allocation");
    black_box(&value);

    let delta = after.physical_mem.saturating_sub(before.physical_mem);
    assert!(
        delta >= min_delta,
        "expected physical memory to increase by at least {min_delta} bytes, got {delta}"
    );
}

pub(super) fn touch_pages(bytes: usize) -> Vec<u8> {
    let mut buf = vec![0u8; bytes];
    let page = page_size().unwrap_or(4096);

    for i in (0..buf.len()).step_by(page) {
        buf[i] = 1;
    }

    black_box(&buf);
    buf
}

#[derive(Clone, Copy, Debug)]
pub(super) struct MemoryUsage {
    pub physical_mem: usize,
    pub virtual_mem: usize,
}

fn page_size() -> Option<usize> {
    unsafe {
        extern "C" {
            fn getpagesize() -> i32;
        }

        let value = getpagesize();
        if value > 0 {
            Some(value as usize)
        } else {
            None
        }
    }
}

fn scan_int(string: &str) -> (usize, usize) {
    let mut out = 0;
    let mut idx = 0;
    let mut chars = string.chars().peekable();
    while let Some(' ') = chars.next_if_eq(&' ') {
        idx += 1;
    }
    for n in chars {
        idx += 1;
        if n.is_ascii_digit() {
            out *= 10;
            out += n as usize - '0' as usize;
        } else {
            break;
        }
    }
    (out, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_some_memory_usage() {
        let usage = memory_usage().expect("expected to read memory usage");
        assert!(usage.physical_mem > 0);
        assert!(usage.virtual_mem > 0);
    }

    #[test]
    fn detects_memory_growth() {
        assert_physical_memory_increases_by_at_least(0, || touch_pages(64 * 1024 * 1024));
    }
}
