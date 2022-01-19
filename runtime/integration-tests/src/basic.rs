use super::*;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn basic_setup() {
        ExtBuilder::default().build().execute_with(|| {});
    }
}
