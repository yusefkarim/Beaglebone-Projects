use std::fs;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::{thread, time};

static GPIO_PATH: &str = "/sys/class/gpio/";

enum Direction { INPUT, OUTPUT }
enum Value { UNDEFINED = -1, LOW = 48, HIGH = 49 }

pub struct InputPin {
    pin_num: u32,
    full_path: String,
    buffer: Option<File>,
    debounce_time: u32,
    last_value: Value,
}

pub struct OutputPin {
    pin_num: u32,
    full_path: String,
    buffer: Option<File>,
    last_value: Value,
    toggle_period: u32,
}

trait GPIO {
    fn set_direction(path: &str, dir: Direction) {
        let dir: &str = match dir {
            Direction::INPUT => "in",
            Direction::OUTPUT => "out",
        };

        let direction_path: &str = &[path, "direction"].concat();
        fs::write(direction_path, dir)
            .expect("Could not write to direction file");
    }

    fn export(pin_num: u32, full_path: &str) {
        if !fs::metadata(&full_path).is_ok() {
            fs::write(GPIO_PATH.to_string() + "export", pin_num.to_string())
                .expect("Could not open export file");
            // If using udev rules, give some time for it to apply permissions
            thread::sleep(time::Duration::from_secs(1));
        }
    }

    fn unexport(pin_num: u32, full_path: &str) {
        if fs::metadata(&full_path).is_ok() {
            fs::write(GPIO_PATH.to_string() + "unexport", pin_num.to_string())
                .expect("Could not open unexport file");
        }
    }

    fn read_value(value_file: &mut File) -> Value {
        let mut one_byte: [u8; 1] = [0; 1];
        value_file.read(&mut one_byte).expect("Unable to read value file");
        if one_byte[0] == Value::LOW as u8 {
            Value::LOW
        } else {
            Value::HIGH
        }
    }
}

impl GPIO for InputPin {}
impl GPIO for OutputPin {}

impl Drop for InputPin {
    fn drop(&mut self) {
        Self::unexport(self.pin_num, &self.full_path);
    }
}

impl Drop for OutputPin {
    fn drop(&mut self) {
        Self::unexport(self.pin_num, &self.full_path);
    }
}

impl InputPin {
    pub fn new(pin_num: u32) -> InputPin {
        let full_path: String = format!("{}gpio{}/", GPIO_PATH, pin_num);
        Self::export(pin_num, &full_path);
        Self::set_direction(&full_path, Direction::INPUT);
        let value_file_path: String = format!("{}value", &full_path);

        InputPin {
            pin_num: pin_num,
            full_path: full_path,
            buffer: Some(OpenOptions::new().read(true)
                                           .open(value_file_path)
                                           .unwrap()),
            last_value: Value::UNDEFINED,
            debounce_time: 0,
        }
    }
}

impl OutputPin {
    pub fn new(pin_num: u32) -> OutputPin {
        let full_path: String = format!("{}gpio{}/", GPIO_PATH, pin_num);
        Self::export(pin_num, &full_path);
        Self::set_direction(&full_path, Direction::OUTPUT);
        let value_file_path: String = format!("{}value", &full_path);

        OutputPin {
            pin_num: pin_num,
            full_path: full_path,
            buffer: Some(OpenOptions::new().read(true)
                                           .write(true)
                                           .open(value_file_path)
                                           .unwrap()),
            last_value: Value::UNDEFINED,
            toggle_period: 100,
        }
    }

    pub fn turn_on(&mut self) {
        self.buffer.as_ref().unwrap().write(&[Value::HIGH as u8])
            .expect("Error writing value");
        self.last_value = Value::HIGH;
    }

    pub fn turn_off(&mut self) {
        self.buffer.as_ref().unwrap().write(&[Value::LOW as u8])
            .expect("Error writing value");
        self.last_value = Value::LOW;
    }

    pub fn toggle(&mut self) {
        match self.last_value {
            Value::LOW => self.turn_on(),
            Value::HIGH => self.turn_off(),
            Value::UNDEFINED => {
                match Self::read_value(self.buffer.as_mut().unwrap()) {
                    Value::LOW => self.turn_on(),
                    _ => self.turn_off(),
                }
            }
        };
    }
}

