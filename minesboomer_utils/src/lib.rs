use minesweeper_core::Point;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct SerializablePoint {
    x: usize,
    y: usize,
}

impl SerializablePoint {
    pub fn new_from_json(json: String) -> Result<SerializablePoint, serde_json::Error> {
        serde_json::from_str(&json)
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl From<Point> for SerializablePoint {
    fn from(point: Point) -> SerializablePoint {
        SerializablePoint { x: point.x, y: point.y }
    }
}

impl From<SerializablePoint> for Point {
    fn from(point: SerializablePoint) -> Point {
        Point { x: point.x, y: point.y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_point() {
        let point = Point::zero();
        let s_point: SerializablePoint = point.into();

        assert_eq!(point.x, s_point.x);
        assert_eq!(point.y, s_point.y);
    }

    #[test]
    fn convert_s_point() {
        let s_point = SerializablePoint { x: 0, y: 0 };
        let point: Point = s_point.into();

        assert_eq!(point.x, s_point.x);
        assert_eq!(point.y, s_point.y);
    }

    #[test]
    fn serialize_s_point() {
        let s_point = SerializablePoint { x: 1, y: 2 };
        let json = s_point.to_json_string();

        assert_eq!(json, "{\"x\":1,\"y\":2}");
    }
}
