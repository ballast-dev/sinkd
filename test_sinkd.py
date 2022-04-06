import os
from pathlib import Path
import shutil
import subprocess

try:
    shutil.rmtree("/home/tony/dmz/client/boom")
    shutil.rmtree("/home/tony/dmz/client/other")
except FileNotFoundError as e:
    print(e)

os.mkdir("/home/tony/dmz/client/boom")
subprocess.run(["touch",
                "/home/tony/dmz/client/boom/file1",
                "/home/tony/dmz/client/boom/file2",
                "/home/tony/dmz/client/boom/file3",
                "/home/tony/dmz/client/boom/file4",
                "/home/tony/dmz/client/boom/file5"])

os.mkdir("/home/tony/dmz/client/other")
subprocess.run(["touch",
                "/home/tony/dmz/client/other/file1",
                "/home/tony/dmz/client/other/file2",
                "/home/tony/dmz/client/other/file3",
                "/home/tony/dmz/client/other/file4",
                "/home/tony/dmz/client/other/file5"])

print("yay?")
