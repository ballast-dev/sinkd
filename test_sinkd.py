import glob
import os
import shutil
import subprocess
import time
from pathlib import Path


def remove_files(tlds: list):
    for tld in tlds:
        wildcard = f"{tld}{os.path.sep}*"
        for d in glob.glob(wildcard):
            try:
                shutil.rmtree(d)
                print("removed ", d)
            except FileNotFoundError as e:
                print(e)


def create_files(folder: Path, filenum: int, delay: float = 0.01):

    os.makedirs(folder)
    for i in range(filenum):
        print(f"touching file{i} with delay:{delay}")
        time.sleep(delay)
        filepath = folder.joinpath(f"file{i}")
        subprocess.run(["touch", filepath])


if __name__ == "__main__":
    TLD = "/home/tony/dmz/client"
    remove_files([TLD])
    boom_folder = Path(TLD, "boom")
    create_files(boom_folder, 3, 0.5)
    print(f"delay:{6}secs")
    time.sleep(6)
    other_folder = Path(TLD, "other")
    create_files(other_folder, 10, 1)
    print("done")
