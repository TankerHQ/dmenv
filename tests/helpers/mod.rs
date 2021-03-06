use std::path::PathBuf;

use ignore::Walk;
use structopt::StructOpt;

use dmenv;

#[derive(Debug)]
pub struct TestApp {
    tmp_dir: tempdir::TempDir,
}

///
/// An instance of dmenv::App designed for testing
///
/// By default, contains a copy of all the files in
/// demo/
impl TestApp {
    pub fn new() -> Self {
        let tmp_dir = tempdir::TempDir::new("test-dmenv").unwrap();
        let test_app = TestApp { tmp_dir };
        test_app.copy_demo_files();
        test_app
    }

    pub fn path(&self) -> PathBuf {
        self.tmp_dir.path().to_path_buf()
    }

    fn copy_demo_files(&self) {
        for result in Walk::new("demo") {
            let entry = result.unwrap();
            if let Some(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let src = entry.path();
                    let name = entry.file_name();
                    let dest = self.path().join(name);
                    std::fs::copy(src, dest).unwrap();
                }
            }
        }
    }

    pub fn override_lock(&self, contents: &str) {
        let path = self.path().join(dmenv::DEV_LOCK_FILENAME);
        std::fs::write(path, contents).unwrap();
    }

    pub fn override_setup_cfg(&self, contents: &str) {
        let path = self.path().join("setup.cfg");
        std::fs::write(path, contents).unwrap();
    }

    pub fn remove_setup_py(&self) {
        self.remove_file("setup.py");
    }

    pub fn remove_setup_cfg(&self) {
        self.remove_file("setup.cfg");
    }

    pub fn remove_dev_lock(&self) {
        self.remove_file(dmenv::DEV_LOCK_FILENAME);
    }

    pub fn remove_prod_lock(&self) {
        self.remove_file(dmenv::PROD_LOCK_FILENAME);
    }

    pub fn run(&self, args: Vec<String>) -> Result<(), dmenv::Error> {
        let mut cmd = vec![];
        cmd.extend(vec!["dmenv".to_string()]);
        let tmp_path: String = self.path().to_string_lossy().into();
        cmd.extend(vec!["--project".to_string(), tmp_path]);
        cmd.extend(args);
        let cmd = dmenv::Command::from_iter_safe(cmd).unwrap();
        dmenv::run_cmd(cmd)
    }

    pub fn assert_run_ok(&self, args: &[&str]) {
        let args = to_string_args(&args);
        self.run(args).unwrap();
    }

    pub fn read_dev_lock(&self) -> String {
        let lock_path = self.path().join(dmenv::DEV_LOCK_FILENAME);
        std::fs::read_to_string(lock_path).unwrap()
    }

    pub fn read_prod_lock(&self) -> String {
        let lock_path = self.path().join(dmenv::PROD_LOCK_FILENAME);
        std::fs::read_to_string(lock_path).unwrap()
    }

    pub fn assert_setup_py(&self) {
        self.assert_file("setup.py");
    }

    pub fn assert_file(&self, name: &str) {
        assert!(self.path().join(name).exists());
    }

    pub fn assert_run_error(&self, args: &[&str]) -> String {
        let args = to_string_args(&args);
        let res = self.run(args);
        res.unwrap_err().to_string()
    }

    pub fn remove_file(&self, name: &str) {
        let path = self.path().join(name);
        std::fs::remove_file(path).unwrap();
    }
}

pub fn to_string_args(args: &[&str]) -> Vec<String> {
    args.iter().map(|x| (*x).to_string()).collect()
}
