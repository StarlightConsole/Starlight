use crate::print;

const COMMAND_PREFIX: &str = "#COMET:";

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub enum Command {
    SetDevice = 0x01,
    RequestBinary = 0x02,
    SendBinary = 0x03
}

impl Command {
    pub fn as_char(&self) -> char {
        *self as u8 as char
    }
}

fn send_command(command: Command, args: Option<&[u8]>) {
    print!("{}", COMMAND_PREFIX);
    print!("{}", command.as_char());

    if let Some(args) = args {
        for arg in args {
            print!("{}", *arg as char);
        }
    }

}

#[allow(unused)]
pub enum Device {
    Starship = 1,
    Starlight = 2,
    StarlightMini = 3
}

pub fn set_device(device: Device) {
    send_command(Command::SetDevice, Some(&[device as u8]))
}

#[allow(unused)]
pub fn request_binary() {
    send_command(Command::RequestBinary, None)
}
