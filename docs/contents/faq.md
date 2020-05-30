# FAQ

#### I'm on Debian, and I've got errors when running `bdist_wheel`

This is an [upstream bug](https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=917006).
As a workaround, you can install virtual environment with `python3 -m pip install virtual environment --user`
and then set the `DMENV_NO_VENV_STDLIB` environment variable to a non-empty value like `1`.

#### Why Rust?

* Because it has excellent support for what we need: manipulate paths and run commands in a cross-platform way
* Because it's my second favorite language
* Because distribution is really easy
* Because by *not* using Python at all `dmenv` is less likely to break if something on your system changes.

#### Why should I use dmenv and not existing tools like pipenv, tox, poetry, flint, ...?

`dmenv` works really well if:

* You already know how `pip` and `python3 -m venv` work
* You don't really mind that `pip` does **not** solve dependencies
* You want to be sure your code can be published to pypi.org or any pip
  mirror if you need to
* You want your code to work with several versions on Python

Compared to other tools, it may be much faster, but at the cost of having
less features, or be harder to use.

Please bear that in mind when considering using `dmenv` for your own project.
