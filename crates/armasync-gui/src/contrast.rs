// SPDX-License-Identifier: AGPL-3.0-only
//! WCAG 2.x contrast ratio helpers (relative luminance, sRGB).

/// Relative luminance of an sRGB colour (components 0–255), per WCAG 2.x.
#[must_use]
pub fn relative_luminance(rgb: [u8; 3]) -> f64 {
    let linear = |component: u8| {
        let value = f64::from(component) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

/// Contrast ratio between two opaque colours (1.0 – 21.0).
#[must_use]
pub fn contrast_ratio(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::contrast_ratio;

    #[test]
    fn black_on_white_is_21() {
        assert!((contrast_ratio([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.01);
        assert!((contrast_ratio([255, 255, 255], [255, 255, 255]) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn known_pair() {
        // #767676 on white is the classic 4.54:1 boundary example.
        let ratio = contrast_ratio([0x76, 0x76, 0x76], [255, 255, 255]);
        assert!((4.5..4.6).contains(&ratio), "{ratio}");
    }
}
