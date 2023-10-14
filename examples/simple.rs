use ru_input_helper::InputManager;

#[derive(Debug, PartialEq, Eq, Hash)]
enum Bind {
    A,
    B,
    C,
}

fn main() {
    let mut input_manager = InputManager::<Bind>::default();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait();
        println!("Received event {event:?}");
        if input_manager.pass_event(&event) {
            if input_manager.pressed(&Bind::A) {
                println!("A pressed");
            }
            if input_manager.pressed(&Bind::B) {
                println!("B pressed");
            }
            if input_manager.pressed(&Bind::C) {
                println!("C pressed");
            }
        }
    });
}
