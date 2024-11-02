#!/usr/bin/env python3
import shlex
import shutil
import subprocess
import time
from pathlib import Path

TLD = Path(__file__).parents[1]
CLIENT_PATH = Path(TLD, "test", "client")
SERVER_PATH = Path(TLD, "test", "server")


def run(cmd, **kwargs) -> subprocess.CompletedProcess:
    if isinstance(cmd, str):
        cmd = shlex.split(cmd)
    print(" ".join(cmd))
    return subprocess.run(cmd, **kwargs, encoding="utf8")


def setup_env():
    CLIENT_PATH.mkdir(exist_ok=True, parents=True)
    SERVER_PATH.mkdir(exist_ok=True, parents=True)


def remove_subfiles(directory: Path):
    if directory:
        for f in directory.glob("*"):
            try:
                if Path(f).is_dir():
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
        # yet I think sinkd doesn't catch these events
        subprocess.run(["touch", filepath])


def spawn_server():
    client = run("./target/debug/sinkd server start -d")
    if client.returncode != 0:
        print("test_sinkd>> ", client.stderr, client.stdout)
        exit(-1)
    print("sucessfully spawned sinkd")

def spawn_client():
    sys_cfg = CLIENT_PATH.joinpath("etc_sinkd.conf")
    usr_cfg = CLIENT_PATH.joinpath("sinkd.conf")
    client = run(
        f"./target/debug/sinkd client start -d -s {sys_cfg} -u {usr_cfg}"
    )
    if client.returncode != 0:
        print("test_sinkd>> ", client.stderr, client.stdout)
        exit(-1)
    print("sucessfully spawned sinkd")


def stop_sinkd():
    run("./target/debug/sinkd client -d stop")
    run("./target/debug/sinkd server -d stop")


def run_situation():
    remove_subfiles(CLIENT_PATH)
    folder1 = Path(CLIENT_PATH, "folder1")
    create_files(folder1, 3, 0.5)
    folder2 = Path(CLIENT_PATH, "folder2")
    create_files(folder2, 10, 1)
    print("==>> Finished client situation <<==")


if __name__ == "__main__":
    setup_env()
    spawn_server()
    spawn_client()
    run_situation()
    stop_sinkd()
