//! Coordinate-space helpers: `CoordValue`, `BoundingBox`, `resolve_relative_coords`.
//!
//! This module is part of the **library** crate so that the CLI layer (`cli/mod.rs`) can use
//! `CoordValue` as a clap argument type.
//!
//! CDP-dependent helpers (`frame_viewport_offset`, `resolve_element_box`) live in the binary
//! crate's `coord_helpers` module (`src/coord_helpers.rs`) to avoid pulling `snapshot` into the
//! library.

use clap::builder::{StringValueParser, TypedValueParser, ValueParserFactory};

// =============================================================================
// CoordValue — custom clap type
// =============================================================================

/// A coordinate value supplied on the command line: either a pixel offset or a percentage.
///
/// Accepted forms:
/// - Pixel: `"10"`, `"-10"`, `"10.5"`, `"0"` → `Pixels(f64)`
/// - Percentage: `"0%"`, `"50%"`, `"100%"`, `"33.33%"` → `Percent(f64)`
///   - Must be in the range `[0.0, 100.0]`; outside values are rejected.
///
/// Percentages without `--relative-to` are rejected at the executor level (not parse time)
/// with exit code 1 and message `"percentage coordinates require --relative-to"`.
#[derive(Debug, Clone, Copy)]
pub enum CoordValue {
    /// Absolute pixel offset (from input like `"10"` or `"-10.5"`).
    Pixels(f64),
    /// Percentage of the `--relative-to` element's dimension (from input like `"50%"`).
    /// Stored as the raw number before the `%` sign (e.g., `50.0` for `"50%"`).
    Percent(f64),
}

/// Parse errors returned by [`CoordValue::parse`].
#[derive(Debug)]
pub struct CoordValueParseError(pub String);

impl std::fmt::Display for CoordValueParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CoordValueParseError {}

impl CoordValue {
    /// Parse a coordinate value from a CLI string.
    ///
    /// # Errors
    ///
    /// Returns [`CoordValueParseError`] if the string is not a valid pixel value or in-range
    /// percentage.
    pub fn parse(s: &str) -> Result<Self, CoordValueParseError> {
        if s.is_empty() {
            return Err(CoordValueParseError(
                "coordinate value must not be empty".to_string(),
            ));
        }

        if let Some(pct_str) = s.strip_suffix('%') {
            // Must be exactly one '%' — no trailing content after the '%'
            if pct_str.contains('%') {
                return Err(CoordValueParseError(format!(
                    "invalid coordinate '{s}': malformed percentage (extra '%')"
                )));
            }
            if pct_str.is_empty() {
                return Err(CoordValueParseError(
                    "invalid coordinate '%': percentage value must have a number before '%'"
                        .to_string(),
                ));
            }
            let value: f64 = pct_str.parse().map_err(|_| {
                CoordValueParseError(format!(
                    "invalid coordinate '{s}': percentage value is not a valid number"
                ))
            })?;
            if !(0.0..=100.0).contains(&value) {
                return Err(CoordValueParseError(format!(
                    "invalid coordinate '{s}': percentage must be in range 0%–100% (got {value}%)"
                )));
            }
            Ok(Self::Percent(value))
        } else {
            // Pixel value — must not contain '%'
            if s.contains('%') {
                return Err(CoordValueParseError(format!(
                    "invalid coordinate '{s}': malformed value"
                )));
            }
            let value: f64 = s.parse().map_err(|_| {
                CoordValueParseError(format!(
                    "invalid coordinate '{s}': not a valid number or percentage"
                ))
            })?;
            Ok(Self::Pixels(value))
        }
    }
}

// =============================================================================
// clap ValueParserFactory integration
// =============================================================================

/// Clap value parser that routes the raw string to [`CoordValue::parse`].
///
/// Using `ValueParserFactory` (rather than a plain function) ensures clap does **not** first
/// try to parse the argument as `f64` before handing it to our code, which would reject
/// `"50%"` before it ever reached `CoordValue::parse`.
#[derive(Clone)]
pub struct CoordValueParser;

impl TypedValueParser for CoordValueParser {
    type Value = CoordValue;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner = StringValueParser::new();
        let s = inner.parse_ref(cmd, arg, value)?;
        CoordValue::parse(&s).map_err(|e| {
            let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
            err.insert(
                clap::error::ContextKind::InvalidValue,
                clap::error::ContextValue::String(e.to_string()),
            );
            err
        })
    }
}

impl ValueParserFactory for CoordValue {
    type Parser = CoordValueParser;

    fn value_parser() -> Self::Parser {
        CoordValueParser
    }
}

// =============================================================================
// BoundingBox
// =============================================================================

/// A bounding box in frame-local coordinates.
///
/// `x`, `y` are the top-left corner; `width` and `height` are the dimensions.
/// All values are in CSS pixels.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// =============================================================================
// resolve_relative_coords — pure coordinate arithmetic
// =============================================================================

/// Resolve a `(CoordValue, CoordValue)` pair to page-global `(x, y)` coordinates.
///
/// - `element_box` must be in **frame-local** coordinates (no frame offset applied).
/// - `frame_offset` is `(0.0, 0.0)` for the main frame.
///
/// # Arithmetic
/// - `Pixels(px)`: `element_box.{x,y} + px + frame_offset`
/// - `Percent(p)`:
///   - `100%` is treated as a special case → `(dimension - 1).max(0)` so the click lands on the
///     **last pixel inside** the element rather than the first pixel outside.
///   - All other percentages map to `(p/100) * dimension`, so `50%` is exactly the midpoint of
///     the element (as users expect when they say "click the center").
/// - Axes are resolved independently (mixed pixel+percent is supported).
#[must_use]
pub fn resolve_relative_coords(
    x: CoordValue,
    y: CoordValue,
    element_box: BoundingBox,
    frame_offset: (f64, f64),
) -> (f64, f64) {
    let x_frame_local = match x {
        CoordValue::Pixels(px) => element_box.x + px,
        CoordValue::Percent(p) => element_box.x + percent_to_offset(p, element_box.width),
    };
    let y_frame_local = match y {
        CoordValue::Pixels(py) => element_box.y + py,
        CoordValue::Percent(p) => element_box.y + percent_to_offset(p, element_box.height),
    };
    (
        x_frame_local + frame_offset.0,
        y_frame_local + frame_offset.1,
    )
}

/// Convert a percentage (`0.0`–`100.0`) to a pixel offset within a dimension.
///
/// `100%` is special-cased to `(dim - 1).max(0)` so the click lands on the last pixel
/// inside the element. Other values use `(p/100) * dim`, so `50%` lands at the exact center.
fn percent_to_offset(p: f64, dim: f64) -> f64 {
    if (p - 100.0).abs() < f64::EPSILON {
        (dim - 1.0).max(0.0)
    } else {
        (p / 100.0) * dim
    }
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- CoordValue::parse ----

    #[test]
    fn parse_integer_pixels() {
        let v = CoordValue::parse("10").unwrap();
        assert!(matches!(v, CoordValue::Pixels(x) if (x - 10.0).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_negative_integer_pixels() {
        let v = CoordValue::parse("-10").unwrap();
        assert!(matches!(v, CoordValue::Pixels(x) if (x - (-10.0)).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_decimal_pixels() {
        let v = CoordValue::parse("10.5").unwrap();
        assert!(matches!(v, CoordValue::Pixels(x) if (x - 10.5).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_negative_decimal_pixels() {
        let v = CoordValue::parse("-10.5").unwrap();
        assert!(matches!(v, CoordValue::Pixels(x) if (x - (-10.5)).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_zero_pixels() {
        let v = CoordValue::parse("0").unwrap();
        assert!(matches!(v, CoordValue::Pixels(x) if x == 0.0));
    }

    #[test]
    fn parse_zero_percent() {
        let v = CoordValue::parse("0%").unwrap();
        assert!(matches!(v, CoordValue::Percent(p) if p == 0.0));
    }

    #[test]
    fn parse_fifty_percent() {
        let v = CoordValue::parse("50%").unwrap();
        assert!(matches!(v, CoordValue::Percent(p) if (p - 50.0).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_hundred_percent() {
        let v = CoordValue::parse("100%").unwrap();
        assert!(matches!(v, CoordValue::Percent(p) if (p - 100.0).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_fractional_percent() {
        let v = CoordValue::parse("33.33%").unwrap();
        assert!(matches!(v, CoordValue::Percent(p) if (p - 33.33).abs() < 0.001));
    }

    #[test]
    fn parse_empty_string_rejected() {
        assert!(CoordValue::parse("").is_err());
    }

    #[test]
    fn parse_non_numeric_rejected() {
        assert!(CoordValue::parse("abc").is_err());
    }

    #[test]
    fn parse_double_percent_rejected() {
        assert!(CoordValue::parse("5%%").is_err());
    }

    #[test]
    fn parse_bare_percent_rejected() {
        assert!(CoordValue::parse("%").is_err());
    }

    #[test]
    fn parse_negative_percent_rejected() {
        assert!(CoordValue::parse("-5%").is_err());
    }

    #[test]
    fn parse_over_100_percent_rejected() {
        assert!(CoordValue::parse("150%").is_err());
    }

    #[test]
    fn parse_100_01_percent_rejected() {
        assert!(CoordValue::parse("100.01%").is_err());
    }

    // Regression test: negative pixel values must parse as Pixels (not rejected)
    #[test]
    fn parse_negative_pixel_regression() {
        let v = CoordValue::parse("-10").unwrap();
        assert!(
            matches!(v, CoordValue::Pixels(x) if (x - (-10.0)).abs() < f64::EPSILON),
            "negative pixel value must parse as Pixels, not be rejected"
        );
    }

    // ---- resolve_relative_coords ----

    fn bbox(x: f64, y: f64, w: f64, h: f64) -> BoundingBox {
        BoundingBox {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn resolve_both_pixels_main_frame() {
        let r = resolve_relative_coords(
            CoordValue::Pixels(10.0),
            CoordValue::Pixels(5.0),
            bbox(100.0, 200.0, 80.0, 32.0),
            (0.0, 0.0),
        );
        assert_eq!(r, (110.0, 205.0));
    }

    #[test]
    fn resolve_both_50_percent_is_center_main_frame() {
        // 50% maps to dim/2 (not (dim-1)/2), so center is exact:
        // x: 100 + 0.5 * 200 = 200
        // y: 200 + 0.5 * 100 = 250
        let r = resolve_relative_coords(
            CoordValue::Percent(50.0),
            CoordValue::Percent(50.0),
            bbox(100.0, 200.0, 200.0, 100.0),
            (0.0, 0.0),
        );
        assert!((r.0 - 200.0).abs() < f64::EPSILON, "x={}", r.0);
        assert!((r.1 - 250.0).abs() < f64::EPSILON, "y={}", r.1);
    }

    #[test]
    fn resolve_0_percent_top_left() {
        let r = resolve_relative_coords(
            CoordValue::Percent(0.0),
            CoordValue::Percent(0.0),
            bbox(100.0, 200.0, 200.0, 100.0),
            (0.0, 0.0),
        );
        // 0% → 0 * (w-1) = 0, so (100, 200)
        assert_eq!(r, (100.0, 200.0));
    }

    #[test]
    fn resolve_100_percent_bottom_right_inclusive() {
        let r = resolve_relative_coords(
            CoordValue::Percent(100.0),
            CoordValue::Percent(100.0),
            bbox(100.0, 200.0, 200.0, 100.0),
            (0.0, 0.0),
        );
        // 100% → 1.0 * (200-1) = 199, so x = 100+199=299
        // 100% → 1.0 * (100-1) = 99, so y = 200+99=299
        assert_eq!(r, (299.0, 299.0));
    }

    #[test]
    fn resolve_mixed_percent_pixels() {
        // element (100,200,200,100), 50% x + 10 pixel y
        let r = resolve_relative_coords(
            CoordValue::Percent(50.0),
            CoordValue::Pixels(10.0),
            bbox(100.0, 200.0, 200.0, 100.0),
            (0.0, 0.0),
        );
        // x: 100 + 0.5 * 200 = 200
        // y: 200 + 10 = 210
        assert!((r.0 - 200.0).abs() < f64::EPSILON, "x={}", r.0);
        assert!((r.1 - 210.0).abs() < f64::EPSILON, "y={}", r.1);
    }

    #[test]
    fn resolve_with_iframe_offset() {
        let r = resolve_relative_coords(
            CoordValue::Percent(50.0),
            CoordValue::Percent(50.0),
            bbox(10.0, 20.0, 80.0, 32.0),
            (50.0, 100.0),
        );
        // x: 10 + 0.5*80 + 50 = 10 + 40 + 50 = 100
        // y: 20 + 0.5*32 + 100 = 20 + 16 + 100 = 136
        assert!((r.0 - 100.0).abs() < f64::EPSILON, "x={}", r.0);
        assert!((r.1 - 136.0).abs() < f64::EPSILON, "y={}", r.1);
    }

    #[test]
    fn resolve_zero_width_element_does_not_panic() {
        let r = resolve_relative_coords(
            CoordValue::Percent(100.0),
            CoordValue::Percent(100.0),
            bbox(50.0, 50.0, 0.0, 0.0),
            (0.0, 0.0),
        );
        // (0-1).max(0) = 0, so result is just (50, 50)
        assert_eq!(r, (50.0, 50.0));
    }

    // ---- clap integration test ----

    // Verify that clap routes "50%" as a raw string to CoordValueParser without
    // first trying to parse it as f64 (which would fail).
    #[test]
    fn clap_routes_50_percent_to_coord_value() {
        use clap::Parser;

        #[derive(Parser)]
        struct MinimalArgs {
            #[arg(value_parser = clap::value_parser!(CoordValue))]
            x: CoordValue,
        }

        let args = MinimalArgs::try_parse_from(["test", "50%"]).expect("clap rejected '50%'");
        assert!(
            matches!(args.x, CoordValue::Percent(p) if (p - 50.0).abs() < f64::EPSILON),
            "expected Percent(50.0), got {:?}",
            args.x
        );
    }

    #[test]
    fn clap_routes_negative_pixels_to_coord_value() {
        use clap::Parser;

        #[derive(Parser)]
        struct MinimalArgs {
            #[arg(value_parser = clap::value_parser!(CoordValue), allow_hyphen_values = true)]
            x: CoordValue,
        }

        // negative pixel values must be accepted (regression: clap might interpret '-' as a flag)
        let args =
            MinimalArgs::try_parse_from(["test", "-10"]).expect("clap rejected negative pixel");
        assert!(
            matches!(args.x, CoordValue::Pixels(px) if (px - (-10.0)).abs() < f64::EPSILON),
            "expected Pixels(-10.0), got {:?}",
            args.x
        );
    }
}
