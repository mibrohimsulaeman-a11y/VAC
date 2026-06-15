pub trait Greeter {
    fn greet(&self) -> String;
}

pub struct Person {
    pub name: String,
}

impl Greeter for Person {
    fn greet(&self) -> String {
        format!("hello {}", self.name)
    }
}

impl Person {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
