#!/bin/bash env python3
###/usr/bin/python3   #don't know if this works

# Sinkd daemon
# will synchronize two folders
# will be invoked on the terminal
import os


local_folder = "/path/to/local_folder"
remote_folder = "/path/to/remote_folder"

def request():
    global local_folder
    global remote_folder
    print("Welcome to sinkd") #text art for SINKD
    # should write all configs to it's unique .sinkd/ folder
    # maybe write a UI for selection
    local_folder = input("Please provide local folder you want to sync: ")
    print("For remote foler access you need user@server.com:~/path/to/folder/")
    remote_folder = input("And the remote folder you want to anchor to: ")

def load_configs (folder_loc):
    set_config = "mkdir " + folder_loc + ".sinkd/"
    os.system(set_config)

def display(arg):
    print(arg)


request()
display(local_folder)
display(remote_folder)
load_configs(local_folder)

#bash_command = "echo YAY!"
#os.system(bash_command)
