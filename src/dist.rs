
use crate::package::Package;

pub struct Dist<'a>{
    pub package: &'a Package<'a>,
    pub version: &'a str,
    pub reference: &'a str,
    pub dist_type: &'a str
}

impl<'a> Dist<'a> {
    pub fn new(package: &'a Package, version: &'a str, reference: &'a str, dist_type: &'a str) -> Self {
        Self {
            package,
            version,
            reference,
            dist_type
        }
    }
}