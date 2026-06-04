use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::profiles::PixelPoint;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PixelSampleRequest {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PixelSample {
    pub color: String,
    pub x: i32,
    pub y: i32,
}

#[cfg(windows)]
pub fn sample_pixel(point: PixelPoint) -> Result<PixelSample, String> {
    use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};

    unsafe {
        let dc = GetDC(std::ptr::null_mut());
        if dc.is_null() {
            return Err("Unable to get screen device context".into());
        }

        let color_ref = GetPixel(dc, point.x, point.y);
        ReleaseDC(std::ptr::null_mut(), dc);

        if color_ref == 0xFFFF_FFFF {
            return Err("Unable to sample pixel".into());
        }

        let r = color_ref & 0xFF;
        let g = (color_ref >> 8) & 0xFF;
        let b = (color_ref >> 16) & 0xFF;

        Ok(PixelSample {
            color: format!("#{r:02x}{g:02x}{b:02x}"),
            x: point.x,
            y: point.y,
        })
    }
}

#[cfg(windows)]
pub fn pick_pixel_from_click(timeout_ms: u64) -> Result<PixelSample, String> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    const VK_LBUTTON: i32 = 0x01;
    let timeout_at = Instant::now() + Duration::from_millis(timeout_ms);
    let left_mouse_down = || unsafe { GetAsyncKeyState(VK_LBUTTON) & 0x8000u16 as i16 != 0 };

    while left_mouse_down() && Instant::now() < timeout_at {
        std::thread::sleep(Duration::from_millis(12));
    }

    while Instant::now() < timeout_at {
        if left_mouse_down() {
            let mut point = POINT { x: 0, y: 0 };
            let ok = unsafe { GetCursorPos(&mut point) };
            if ok == 0 {
                return Err("Unable to read cursor position".into());
            }
            while left_mouse_down() && Instant::now() < timeout_at {
                std::thread::sleep(Duration::from_millis(12));
            }
            return sample_pixel(PixelPoint {
                x: point.x,
                y: point.y,
            });
        }

        std::thread::sleep(Duration::from_millis(12));
    }

    Err("Pixel picker timed out before a click was captured".into())
}

#[cfg(not(windows))]
pub fn sample_pixel(point: PixelPoint) -> Result<PixelSample, String> {
    Ok(PixelSample {
        color: "#34d399".into(),
        x: point.x,
        y: point.y,
    })
}

#[cfg(not(windows))]
pub fn pick_pixel_from_click(_timeout_ms: u64) -> Result<PixelSample, String> {
    Ok(PixelSample {
        color: "#34d399".into(),
        x: 640,
        y: 360,
    })
}

pub fn color_matches(sample: &str, target: &str, tolerance: u8) -> bool {
    let Some(sample_rgb) = parse_hex_color(sample) else {
        return false;
    };
    let Some(target_rgb) = parse_hex_color(target) else {
        return false;
    };

    sample_rgb
        .iter()
        .zip(target_rgb.iter())
        .all(|(sample_channel, target_channel)| {
            sample_channel.abs_diff(*target_channel) <= tolerance
        })
}

pub fn is_valid_hex_color(value: &str) -> bool {
    parse_hex_color(value).is_some()
}

pub fn sample_rule_points(center: PixelPoint, adjacent: bool) -> Vec<PixelPoint> {
    if !adjacent {
        return vec![center];
    }

    vec![
        center,
        PixelPoint {
            x: center.x - 1,
            y: center.y,
        },
        PixelPoint {
            x: center.x + 1,
            y: center.y,
        },
        PixelPoint {
            x: center.x,
            y: center.y - 1,
        },
        PixelPoint {
            x: center.x,
            y: center.y + 1,
        },
    ]
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some([r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_matching_handles_exact_and_near_values() {
        assert!(color_matches("#34d399", "#34d399", 0));
        assert!(color_matches("#34d39a", "#34d399", 2));
        assert!(!color_matches("#34d3aa", "#34d399", 2));
    }

    #[test]
    fn adjacent_sampling_returns_cross_pattern() {
        let points = sample_rule_points(PixelPoint { x: 10, y: 20 }, true);
        assert_eq!(points.len(), 5);
        assert!(points.contains(&PixelPoint { x: 10, y: 20 }));
    }

    #[test]
    fn hex_color_validation_rejects_malformed_values() {
        assert!(is_valid_hex_color("#34d399"));
        assert!(!is_valid_hex_color("#xyzxyz"));
        assert!(!is_valid_hex_color("#fff"));
    }
}
