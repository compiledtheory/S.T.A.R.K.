use crate::sensors::MotionEvent;
use serde::Serialize;

const HOME_WIDTH_MM: f32 = 9500.0;
const HOME_HEIGHT_MM: f32 = 4000.0;

#[derive(Serialize)]
pub struct Positions {
    pub positions: Vec<Position>
}

impl Positions {
    pub fn create(event: MotionEvent) -> Self {
        let mut positions = Vec::<Position>::with_capacity(event.targets.len());
        for target in event.targets {
            positions.push(Position::normalized(target.x, target.y));
        }

        Self { positions }
    }

    pub fn as_string(&self) -> String {
        serde_json::to_string(&self.positions).unwrap_or_else(|_| "[]".to_string())
    }
}

#[derive(Serialize, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn normalized(x_mm: i16, y_mm: i16) -> Self {
        let x_clamped = (x_mm as f32).clamp(0.0, HOME_WIDTH_MM);
        let y_clamped = (y_mm as f32).clamp(0.0, HOME_HEIGHT_MM);

        Self {
            x: x_clamped / HOME_WIDTH_MM,
            y: y_clamped / HOME_HEIGHT_MM,
        }
    }
}