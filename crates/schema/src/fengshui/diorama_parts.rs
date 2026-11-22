//! 空間風水結果頁縮景零件清單——由 `scripts/sprite-stack/fengshui_parts.py` 產生，**勿手改**。
//!
//! 切片在 `crates/web/assets/stack/fs/<room>/<id>/NN.webp`（只有 `layers` 列出的層）。
//! 改零件後重跑產生器並跑 `--check`；答案→零件對照在 `diorama.rs`。

use super::Room;

/// 零件切片的平面格數（寬＝高）。
pub const DIORAMA_GRID: usize = 40;

/// 一個零件：房型、id（目錄名）、有切片的全域層號（由低到高）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DioramaPart {
    pub room: Room,
    pub id: &'static str,
    pub layers: &'static [u8],
}

/// 全部零件；同房型內的順序即同層內的繪製順序。
pub const DIORAMA_PARTS: &[DioramaPart] = &[
    DioramaPart {
        room: Room::Bedroom,
        id: "base",
        layers: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "bed",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "bed-back-wall",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "bed-back-window",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "bed-back-door",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "bed-floating",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "door-diag",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "door-foot",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "door-behind",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "door-opposite-diag",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "door-opposite-foot",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "door-opposite-behind",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "beam",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "shelf-over",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "mirror",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "corner",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "window-glow",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "clutter-some",
        layers: &[1, 2, 3, 4, 5, 6],
    },
    DioramaPart {
        room: Room::Bedroom,
        id: "clutter-lots",
        layers: &[1, 2, 3, 4, 5, 6, 7],
    },
    DioramaPart {
        room: Room::Study,
        id: "base",
        layers: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Study,
        id: "desk",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
    },
    DioramaPart {
        room: Room::Study,
        id: "chair",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Study,
        id: "back-wall",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Study,
        id: "back-window",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Study,
        id: "back-door",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Study,
        id: "door-diag",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Study,
        id: "door-front",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Study,
        id: "door-behind",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Study,
        id: "beam",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    },
    DioramaPart {
        room: Room::Study,
        id: "front-open",
        layers: &[1],
    },
    DioramaPart {
        room: Room::Study,
        id: "front-wall",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    },
    DioramaPart {
        room: Room::Study,
        id: "corner",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Study,
        id: "sun-patch",
        layers: &[1, 9],
    },
    DioramaPart {
        room: Room::Study,
        id: "clutter-some",
        layers: &[1, 2, 3, 4, 5, 6],
    },
    DioramaPart {
        room: Room::Study,
        id: "clutter-lots",
        layers: &[1, 2, 3, 4, 5, 6, 7],
    },
    DioramaPart {
        room: Room::Office,
        id: "base",
        layers: &[0],
    },
    DioramaPart {
        room: Room::Office,
        id: "desk",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    },
    DioramaPart {
        room: Room::Office,
        id: "chair",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Office,
        id: "back-wall",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    },
    DioramaPart {
        room: Room::Office,
        id: "back-aisle",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
    },
    DioramaPart {
        room: Room::Office,
        id: "back-low",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8],
    },
    DioramaPart {
        room: Room::Office,
        id: "door-front",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Office,
        id: "boss-desk",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Office,
        id: "front-open",
        layers: &[1],
    },
    DioramaPart {
        room: Room::Office,
        id: "front-partition",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Office,
        id: "beam",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    },
    DioramaPart {
        room: Room::Office,
        id: "lamp-tube",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    },
    DioramaPart {
        room: Room::Office,
        id: "corner",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    DioramaPart {
        room: Room::Office,
        id: "window-side",
        layers: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    DioramaPart {
        room: Room::Office,
        id: "clutter-some",
        layers: &[1, 2, 3, 4, 5, 6],
    },
    DioramaPart {
        room: Room::Office,
        id: "clutter-lots",
        layers: &[1, 2, 3, 4, 5, 6, 7],
    },
];
