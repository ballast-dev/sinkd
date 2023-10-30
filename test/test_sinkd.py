#!/usr/bin/env python
import shutil
import shlex
import subprocess
import time
from pathlib import Path
import multiprocessing as mp


TLD = Path(__file__).parents[1]
SYSTEM_CFG = TLD.joinpath("test", "etc_sinkd.conf")
USER_CFG = TLD.joinpath("test", "sinkd.conf")

ROOT_PATH = Path(TLD, "test", "sinkd_dmz")
CLIENT_PATH = Path(ROOT_PATH, "client")
SERVER_PATH = Path(ROOT_PATH, "server")


def run(cmd, **kwargs) -> subprocess.CompletedProcess:
    if type(cmd) is str:
        cmd = shlex.split(cmd)

    return subprocess.run(cmd, **kwargs, encoding="utf8")


def setup_env():
    ROOT_PATH.mkdir(exist_ok=True, parents=True)
    CLIENT_PATH.mkdir(exist_ok=True)
    SERVER_PATH.mkdir(exist_ok=True)


def remove_subfiles(directory: Path):
    if directory:
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
        # yet I think sinkd doesn't catch these events
        subprocess.run(["touch", filepath])


def spawn_sinkd():
    client = run(f"./target/debug/sinkd --debug -s {SYSTEM_CFG} -u {USER_CFG} start --client")
    if client.returncode != 0:
        print("test_sinkd>> ", client.stderr, client.stdout)
        exit(-1)
    print("sucessfully spawned sinkd")


def stop_sinkd():
    kilt_sinkd = run("sudo pkill sinkd")
    if kilt_sinkd.returncode != 0:
        print("trouble pkilling sinkd")
    else:
        print("succeeded in stoping sinkd daemon")


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
    stop_sinkd()
