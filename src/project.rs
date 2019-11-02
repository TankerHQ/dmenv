use std::path::PathBuf;

use crate::cmd::*;
use crate::dependencies::FrozenDependency;
use crate::error::*;
use crate::operations;
use crate::paths::{Paths, PathsResolver};
use crate::python_info::PythonInfo;
use crate::run::VenvRunner;
use crate::settings::Settings;

#[derive(Debug)]
pub struct Metadata {
    pub dmenv_version: String,
    pub python_platform: String,
    pub python_version: String,
}

#[derive(Debug)]
pub struct Project {
    python_info: PythonInfo,
    settings: Settings,
    paths: Paths,
    venv_runner: VenvRunner,
}

#[derive(Debug)]
pub enum PostInstallAction {
    RunSetupPyDevelop,
    None,
}

#[derive(Debug, Copy, Clone)]
pub enum ProcessScriptsMode {
    Safe,
    Override,
}

impl Project {
    pub fn new(
        project_path: PathBuf,
        python_info: PythonInfo,
        settings: Settings,
    ) -> Result<Self, Error> {
        let python_version = python_info.version.clone();
        let paths_resolver = PathsResolver::new(project_path.clone(), python_version, &settings);
        let paths = paths_resolver.paths()?;
        let venv_runner = VenvRunner::new(&project_path, &paths.venv);
        Ok(Project {
            python_info,
            settings,
            paths,
            venv_runner,
        })
    }

    /// Clean virtualenv. No-op if the virtualenv does not exist
    pub fn clean_venv(&self) -> Result<(), Error> {
        operations::venv::clean(self.paths.venv.clone())
    }

    /// Create a new virtualenv
    //
    // Notes:
    // * The path comes from PathsResolver.paths()
    // * Called by `ensure_venv()` *if* the path does not exist
    fn create_venv(&self) -> Result<(), Error> {
        operations::venv::create(&self.paths.venv, &self.python_info, &self.settings)
    }

    /// Make sure the virtualenv exists, or return an error
    //
    // Note: this must be called by any method that requires the
    // virtualenv to exist, like `show_deps` or `run`:
    // this ensures that error messages printed when the
    // virtualenv does not exist are consistent.
    fn expect_venv(&self) -> Result<(), Error> {
        operations::venv::expect(&self.paths.venv)
    }

    pub fn upgrade_pip(&self) -> Result<(), Error> {
        print_info_2("Upgrading pip");
        let cmd = &["python", "-m", "pip", "install", "pip", "--upgrade"];
        self.venv_runner
            .run(cmd)
            .map_err(|_| Error::UpgradePipError {})
    }

    /// Show the dependencies inside the virtualenv.
    // Note: Run `pip list` so we get what's *actually* installed, not just
    // the contents of the lock file
    pub fn show_deps(&self) -> Result<(), Error> {
        self.venv_runner.run(&["python", "-m", "pip", "list"])
    }

    /// Show the resolved virtualenv path.
    //
    // See `PathsResolver.paths()` for details
    pub fn show_venv_path(&self) -> Result<(), Error> {
        println!("{}", self.paths.venv.display());
        Ok(())
    }

    /// Same has `show_venv_path`, but add the correct subfolder
    /// (`bin` on Linux and macOS, `Scripts` on Windows).
    pub fn show_venv_bin_path(&self) -> Result<(), Error> {
        self.expect_venv()?;
        let bin_path = self.venv_runner.binaries_path();
        println!("{}", bin_path.display());
        Ok(())
    }

    pub fn show_outdated(&self) -> Result<(), Error> {
        #[rustfmt::skip]
        let cmd = &[
            "python", "-m", "pip",
            "list", "--outdated",
            "--format", "columns",
        ];
        self.venv_runner.run(cmd)
    }

    // Re-generate a clean lock:
    //   - clean the virtualenv
    //   - re-create it from scratch, while
    //     making sure no package is updated,
    //     hence the use of `pip install --constraint`
    //     in `self.install_editable_with_constraint()`
    //  - re-generate the lock by only keeping existing dependencies:
    //    see `operations::lock::tidy()`
    pub fn tidy(&self) -> Result<(), Error> {
        if std::env::var("VIRTUAL_ENV").is_ok() {
            // Workaround for https://github.com/TankerHQ/dmenv/issues/110
            return Err(new_error(
                "Please exit the virtualenv before running `dmenv tidy`".to_string(),
            ));
        }
        self.clean_venv()?;
        self.create_venv()?;
        self.install_editable_with_constraint()?;
        let metadata = &self.metadata();
        let frozen_deps = self.get_frozen_deps()?;
        operations::lock::tidy(&self.paths.lock, frozen_deps, &metadata)
    }

    fn install_editable_with_constraint(&self) -> Result<(), Error> {
        let lock_path = &self.paths.lock;
        let message = format!(
            "Installing deps from setup.py, constrained by {}",
            lock_path.display()
        );
        print_info_2(&message);
        let lock_path_str = lock_path.to_string_lossy();
        let mut cmd = self.get_install_editable_cmd().to_vec();
        cmd.extend(&["--constraint", &lock_path_str]);
        self.venv_runner.run(&cmd)
    }

    fn get_install_editable_cmd(&self) -> [&str; 6] {
        let extra = if self.settings.production {
            ".[prod]"
        } else {
            ".[dev]"
        };
        ["python", "-m", "pip", "install", "--editable", extra]
    }

    fn metadata(&self) -> Metadata {
        let dmenv_version = env!("CARGO_PKG_VERSION");
        let python_platform = &self.python_info.platform;
        let python_version = &self.python_info.version;
        Metadata {
            dmenv_version: dmenv_version.to_string(),
            python_platform: python_platform.to_string(),
            python_version: python_version.to_string(),
        }
    }

    /// Get the list of the *actual* deps in the virtualenv by calling `pip freeze`.
    fn get_frozen_deps(&self) -> Result<Vec<FrozenDependency>, Error> {
        let freeze_output = self.run_pip_freeze()?;
        // First, collect all the `pip freeze` lines into frozen dependencies
        let deps: Result<Vec<_>, _> = freeze_output
            .lines()
            .map(|x| FrozenDependency::from_string(x.into()))
            .collect();
        let deps = deps?;
        // Then filter out pkg-resources: this works around a Debian bug in pip:
        // https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=871790
        let res: Vec<_> = deps
            .into_iter()
            .filter(|x| x.name != "pkg-resources")
            .collect();
        Ok(res)
    }

    fn run_pip_freeze(&self) -> Result<String, Error> {
        #[rustfmt::skip]
        let cmd = &[
            "python", "-m", "pip", "freeze",
            "--exclude-editable",
            "--all",
            "--local",
        ];
        self.venv_runner.get_output(cmd)
    }
}
