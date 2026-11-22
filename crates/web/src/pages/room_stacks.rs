//! 選房間按鈕縮景的切片層數——由 `scripts/sprite-stack/rooms.py` 產生，**勿手改**。
//!
//! 切片在 `crates/web/assets/stack/room-<wire>/NN.webp`。

use ft_schema::fengshui::Room;

/// 切片的平面格數（寬＝高）。
pub const ROOM_STACK_GRID: usize = 36;

/// 各房型的層數。
pub const ROOM_STACK_LAYERS: [(Room, usize); 3] =
    [(Room::Bedroom, 13), (Room::Study, 16), (Room::Office, 16)];
