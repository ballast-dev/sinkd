#!/usr/bin/env python
import shutil
import subprocess
import time
from pathlib import Path

ROOT_PATH = Path("sinkd_dmz")
CLIENT_PATH = Path(ROOT_PATH, "client")
SERVER_PATH = Path(ROOT_PATH, "server")

def setup_env():
    ROOT_PATH.mkdir(exist_ok=True)
    CLIENT_PATH.mkdir(exist_ok=True)
    SERVER_PATH.mkdir(exist_ok=True)

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
    setup_env()
    run_client_situation()
