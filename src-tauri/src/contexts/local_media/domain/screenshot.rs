#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LogicalSelection {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisplayGeometry {
    pub(crate) logical_origin_x: i32,
    pub(crate) logical_origin_y: i32,
    pub(crate) logical_width: u32,
    pub(crate) logical_height: u32,
    pub(crate) physical_width: u32,
    pub(crate) physical_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhysicalSelection {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

pub(crate) fn map_screenshot_selection(
    selection: LogicalSelection,
    display: DisplayGeometry,
    minimum: f64,
) -> Option<PhysicalSelection> {
    let values = [selection.x, selection.y, selection.width, selection.height];
    if !values.iter().all(|value| value.is_finite())
        || selection.x < 0.0
        || selection.y < 0.0
        || selection.width < minimum
        || selection.height < minimum
        || selection.x + selection.width > f64::from(display.logical_width)
        || selection.y + selection.height > f64::from(display.logical_height)
        || display.logical_width == 0
        || display.logical_height == 0
        || display.physical_width == 0
        || display.physical_height == 0
    {
        return None;
    }
    let scale_x = f64::from(display.physical_width) / f64::from(display.logical_width);
    let scale_y = f64::from(display.physical_height) / f64::from(display.logical_height);
    let x = (selection.x * scale_x).floor() as u32;
    let y = (selection.y * scale_y).floor() as u32;
    let right = ((selection.x + selection.width) * scale_x).ceil() as u32;
    let bottom = ((selection.y + selection.height) * scale_y).ceil() as u32;
    let right = right.min(display.physical_width);
    let bottom = bottom.min(display.physical_height);
    Some(PhysicalSelection {
        x,
        y,
        width: right.checked_sub(x)?,
        height: bottom.checked_sub(y)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> DisplayGeometry {
        DisplayGeometry {
            logical_origin_x: -1920,
            logical_origin_y: 0,
            logical_width: 1280,
            logical_height: 720,
            physical_width: 1920,
            physical_height: 1080,
        }
    }

    #[test]
    fn maps_hidpi_coordinates_without_losing_edge_pixels() {
        let mapped = map_screenshot_selection(
            LogicalSelection {
                x: 10.2,
                y: 20.2,
                width: 100.1,
                height: 80.1,
            },
            geometry(),
            8.0,
        );
        assert_eq!(
            mapped,
            Some(PhysicalSelection {
                x: 15,
                y: 30,
                width: 151,
                height: 121
            })
        );
    }

    #[test]
    fn negative_desktop_origin_does_not_change_overlay_local_coordinates() {
        let mapped = map_screenshot_selection(
            LogicalSelection {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            },
            geometry(),
            8.0,
        );
        assert_eq!(mapped.map(|value| value.width), Some(1920));
    }

    #[test]
    fn rejects_non_finite_small_and_cross_display_rectangles() {
        for selection in [
            LogicalSelection {
                x: f64::NAN,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
            LogicalSelection {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 20.0,
            },
            LogicalSelection {
                x: 1270.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
        ] {
            assert_eq!(map_screenshot_selection(selection, geometry(), 8.0), None);
        }
    }
}
