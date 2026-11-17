// Integration test for SigmaOS-apt-utils
// Verifies the crate builds and links as a dependency.
#![allow(unused_imports)]

use apt_utils as _;

#[test]
fn crate_links_and_runs() {
    assert_eq!(2 + 2, 4);
}
