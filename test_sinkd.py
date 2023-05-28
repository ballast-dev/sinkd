#!/usr/bin/env python
import shutil
import subprocess
import time
from pathlib import Path

CLIENT_PATH = Path("dmz", "client")
SERVER_PATH = Path("dmz", "server")


def remove_subfiles(directory: Path):
    for f in directory.glob("*"):
        try:
            shutil.rmtree(f)
            print("removed ", f)
        except FileNotFoundError as e:
            print(f"File not found: {e}")


def create_files(folder: Path, num_of_files: int, delay: float=0.01):
    folder.mkdir(exist_ok=True)
    for i in range(num_of_files):
        print(f"touching file{i} with delay:{delay}")
        time.sleep(delay)
        filepath = folder.joinpath(f"file{i}")
        subprocess.run(["touch", filepath])


def run_client_situation(root_path: Path):
    tld = root_path.joinpath(CLIENT_PATH)
    remove_subfiles(tld)
    boom_folder = Path(tld, "boom")
    create_files(boom_folder, 3, 0.5)
    # print(f"delay:{6}secs")
    # time.sleep(6)
    other_folder = Path(tld, "other")
    create_files(other_folder, 10, 1)
    print("==>> Finished client situation <<==")


if __name__ == "__main__":
    userland = Path("~").expanduser()
    run_client_situation(userland)
