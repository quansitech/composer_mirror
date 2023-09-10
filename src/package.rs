pub struct Package<'a> {
    pub vendor: &'a str,
    pub package: &'a str,
    pub full_name: String
}

impl<'a> Package<'a> {
    pub fn new(vendor: &'a str, package: &'a str) -> Self {
        Self {
            vendor,
            package,
            full_name: format!("{}/{}", vendor, package)
        }
    }
}