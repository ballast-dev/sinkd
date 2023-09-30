#!/usr/bin/env python
import shutil
import shlex
import subprocess
import time
from pathlib import Path
import multiprocessing as mp


TLD = None
CLIENT_PATH = None
SERVER_PATH = None


def run(cmd, **kwargs) -> subprocess.CompletedProcess:
    if type(cmd) is str:
        cmd = shlex.split(cmd)

    return subprocess.run(cmd, **kwargs, encoding="utf8")


def setup_env():
    global TLD, CLIENT_PATH, SERVER_PATH
    TLD = run("git rev-parse --show-toplevel", capture_output=True).strip("\n")
    print(TLD)
    ROOT_PATH = Path(TLD, "test", "sinkd_dmz").mkdir(exist_ok=True)
    CLIENT_PATH = Path(ROOT_PATH, "client").mkdir(exist_ok=True)
    SERVER_PATH = Path(ROOT_PATH, "server").mkdir(exist_ok=True)


def remove_subfiles(directory: Path):
    for f in directory.glob("*"):
        try:
            shutil.rmtree(f)
            print("removed ", f)
        except FileNotFoundError as e:
            print(f"File not found: {e}")


def create_files(folder: Path, num_of_files: int, delay: float = 0.01):
    folder.mkdir(exist_ok=True)
    for i in range(num_of_files):
        print(f"touching file{i} with delay:{delay}")
        time.sleep(delay)
        filepath = folder.joinpath(f"file{i}")
        # touching changes access time, which should be an event
        # yet I sinkd, doesn't catch these events
        subprocess.run(["touch", filepath])


def spawn_sinkd():
    pass


def run_client_situation():
    remove_subfiles(CLIENT_PATH)
    boom_folder = Path(CLIENT_PATH, "boom")
    create_files(boom_folder, 3, 0.5)
    # print(f"delay:{6}secs")
    # time.sleep(6)
    other_folder = Path(CLIENT_PATH, "other")
    create_files(other_folder, 10, 1)
    print("==>> Finished client situation <<==")


if __name__ == "__main__":
    # ls = run("ls -la", capture_output=True)
    # for line in ls.stdout.splitlines():
    #     print(f"gotcha {line}")
    # run("printenv")
    setup_env()
    spawn_sinkd()
    run_client_situation()
