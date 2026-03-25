use crate::nes::{
    event::{EmulationEvent, NesEvent, NesEventProxy},
};
use egui::{
    Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2,
    epaint::CircleShape,
};
use tetanes_core::input::{JoypadBtn, Player};
use winit::event::ElementState;

#[derive(Debug, Default)]
pub struct VirtualGamepad;

impl VirtualGamepad {
    pub fn new() -> Self {
        Self
    }

    pub fn ui(&mut self, ui: &mut Ui, event_proxy: &NesEventProxy) {
        let rect = ui.max_rect();
        let painter = ui.painter().clone();
        let color = Color32::from_black_alpha(150);
        let stroke = Stroke::new(2.0, Color32::WHITE);
        let pressed_color = Color32::from_white_alpha(50);

        // Layout constants
        // Scale based on screen width/height to be somewhat responsive?
        // simple fixed pixel sizes for now.
        let radius = 80.0; // D-Pad radius
        let btn_radius = 40.0; // Action button radius
        let padding = 40.0;
        let bottom_y = rect.max.y - padding - radius;
        let left_x = rect.min.x + padding + radius;
        let right_x = rect.max.x - padding - radius;
        
        let dpad_center = Pos2::new(left_x, bottom_y);
        let ab_center = Pos2::new(right_x - 30.0, bottom_y);

        // D-pad
        self.dpad(ui, &painter, dpad_center, radius, color, stroke, event_proxy);

        // A/B Buttons
        let a_pos = ab_center + Vec2::new(btn_radius * 1.5, 0.0);
        let b_pos = ab_center - Vec2::new(btn_radius * 1.5, 0.0);
        
        self.button(
            ui, &painter, "A", a_pos, btn_radius, color, stroke, pressed_color,
            JoypadBtn::A, event_proxy
        );
         self.button(
            ui, &painter, "B", b_pos, btn_radius, color, stroke, pressed_color,
            JoypadBtn::B, event_proxy
        );

        // Start/Select
        let center_x = rect.center().x;
        let start_pos = Pos2::new(center_x + 40.0, rect.max.y - 40.0);
        let select_pos = Pos2::new(center_x - 40.0, rect.max.y - 40.0);

        self.button(
            ui, &painter, "Start", start_pos, 25.0, color, stroke, pressed_color,
            JoypadBtn::Start, event_proxy
        );
        self.button(
            ui, &painter, "Sel", select_pos, 25.0, color, stroke, pressed_color,
            JoypadBtn::Select, event_proxy
        );

    }

    #[allow(clippy::too_many_arguments)]
    fn button(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        label: &str,
        pos: Pos2,
        radius: f32,
        color: Color32,
        stroke: Stroke,
        pressed_color: Color32,
        btn: JoypadBtn,
        event_proxy: &NesEventProxy,
    ) {
        let rect = Rect::from_center_size(pos, Vec2::splat(radius * 2.0));
        let id = Id::new(label);
        let response = ui.interact(rect, id, Sense::drag());
        let is_down = response.is_pointer_button_down_on();

        // Track state across frames so Pressed and Released are always in separate frames.
        // Sending both in the same frame would cause the emulation to drain both events
        // before clocking a NES frame, making the NES never see the press.
        let state_id = Id::new(("btn_state", label));
        ui.memory_mut(|mem| {
            let was_down: bool = mem.data.get_temp(state_id).unwrap_or(false);
            if is_down != was_down {
                let state = if is_down { ElementState::Pressed } else { ElementState::Released };
                event_proxy.event(NesEvent::Emulation(EmulationEvent::Joypad((
                    Player::One,
                    btn,
                    state,
                ))));
                mem.data.insert_temp(state_id, is_down);
            }
        });

        painter.add(Shape::Circle(CircleShape {
            center: pos,
            radius,
            fill: if is_down { pressed_color } else { color },
            stroke,
        }));
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(radius * 0.6),
            Color32::WHITE,
        );
    }
    
    // Simplification for D-Pad
    #[allow(clippy::too_many_arguments)]
    fn dpad(&self, ui: &mut Ui, painter: &Painter, center: Pos2, radius: f32, color: Color32, stroke: Stroke, event_proxy: &NesEventProxy) {
        // Draw background
        painter.add(Shape::Circle(CircleShape {
            center,
            radius,
            fill: color,
            stroke,
        }));
        
        // Invisible interaction area
        let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
        let id = Id::new("dpad");
        let response = ui.interact(rect, id, Sense::drag());

        let mut pressed_up = false;
        let mut pressed_down = false;
        let mut pressed_left = false;
        let mut pressed_right = false;

        if response.dragged() || response.is_pointer_button_down_on() {
             if let Some(pos) = response.interact_pointer_pos() {
                 let delta = pos - center;
                 let dist = delta.length();
                 if dist > 10.0 { // deadzone
                     let angle = delta.y.atan2(delta.x); // -PI to PI
                     // Right: -PI/4 to PI/4
                     // Down: PI/4 to 3PI/4
                     // Left: 3PI/4 to PI or -PI to -3PI/4
                     // Up: -3PI/4 to -PI/4
                     
                     let pi = std::f32::consts::PI;
                     
                     if angle > -pi/4.0 && angle < pi/4.0 {
                         pressed_right = true;
                     } else if angle >= pi/4.0 && angle <= 3.0*pi/4.0 {
                         pressed_down = true;
                     } else if angle >= -3.0*pi/4.0 && angle <= -pi/4.0 {
                         pressed_up = true;
                     } else {
                         pressed_left = true;
                     }
                 }
             }
        }
        
        // We need to track state change.
        // But egui is immediate. 
        // We can send Pressed/Released every frame? No, that floods events.
        // We can check if state changed in `self`? `VirtualGamepad` stores no state.
        // But `response.changed()`? No.
        
        // Hack: Send Pressed every frame, input system should handle idempotency?
        // tetanes Joypad event usually sets the state in `Gamepads`.
        // If we send Pressed, it sets the bit.
        // If we send Released, it clears it.
        // If we don't send anything, the state remains?
        // `gilrs` sends events on change.
        
        // Problem: If I press UP, I send Pressed. Next frame I still press UP. If I send Pressed again, it's fine.
        // But if I stop pressing UP, I must send Released.
        // Since I don't store state, I don't know if I was pressing UP last frame.
        
        // Solution: Use `ui.memory()` to store dpad state.
        ui.memory_mut(|mem| {
             let id = Id::new("dpad_state");
             let prev: u8 = mem.data.get_temp(id).unwrap_or(0);
             let mut current: u8 = 0;
             if pressed_up { current |= 1; }
             if pressed_down { current |= 2; }
             if pressed_left { current |= 4; }
             if pressed_right { current |= 8; }
             
             if current != prev {
                 // Diff and send events
                 let up_changed = (current & 1) != (prev & 1);
                 let down_changed = (current & 2) != (prev & 2);
                 let left_changed = (current & 4) != (prev & 4);
                 let right_changed = (current & 8) != (prev & 8);
                 
                 if up_changed {
                     event_proxy.event(NesEvent::Emulation(EmulationEvent::Joypad((Player::One, JoypadBtn::Up, if pressed_up { ElementState::Pressed } else { ElementState::Released }))));
                 }
                 if down_changed {
                      event_proxy.event(NesEvent::Emulation(EmulationEvent::Joypad((Player::One, JoypadBtn::Down, if pressed_down { ElementState::Pressed } else { ElementState::Released }))));
                 }
                 if left_changed {
                      event_proxy.event(NesEvent::Emulation(EmulationEvent::Joypad((Player::One, JoypadBtn::Left, if pressed_left { ElementState::Pressed } else { ElementState::Released }))));
                 }
                 if right_changed {
                      event_proxy.event(NesEvent::Emulation(EmulationEvent::Joypad((Player::One, JoypadBtn::Right, if pressed_right { ElementState::Pressed } else { ElementState::Released }))));
                 }
                 
                 mem.data.insert_temp(id, current);
             }
        });
        
        // Draw the dpad
        let arrow_offset = radius * 0.6;
        let highlight_color = Color32::from_white_alpha(50);
        let directions = [
            (pressed_up, center - Vec2::new(0.0, arrow_offset), "⬆"),
            (pressed_down, center + Vec2::new(0.0, arrow_offset), "⬇"),
            (pressed_left, center - Vec2::new(arrow_offset, 0.0), "⬅"),
            (pressed_right, center + Vec2::new(arrow_offset, 0.0), "➡"),
        ];

        for (pressed, pos, symbol) in directions {
            if pressed {
                painter.add(Shape::Circle(CircleShape {
                    center: pos,
                    radius: radius * 0.35,
                    fill: highlight_color,
                    stroke: Stroke::NONE,
                }));
            }
            painter.text(
                pos,
                Align2::CENTER_CENTER,
                symbol,
                FontId::proportional(radius * 0.4),
                Color32::WHITE,
            );
        }
    }
}
