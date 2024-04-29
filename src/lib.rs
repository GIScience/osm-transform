mod io;

pub fn hello() {
    println!("Hello, this ist rusty!");
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        dbg!(hello())
    }
}