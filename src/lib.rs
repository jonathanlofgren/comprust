pub mod huffman;

pub fn hello_there(name: &str) {
    println!("Hello {}!", name)
}


pub mod useless {
    pub fn noop() -> i32 {
        10
    }
}