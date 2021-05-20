#!/bin/bash env python3
###/usr/bin/python3   #don't know if this works

# Sinkd daemon
# will synchronize two folders
# will be invoked on the terminal
import subprocess


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
    if (folder_loc):
        # set the last character to '/'
    cfg_dir = folder_loc + ".sinkd/"
    # mk_cfg_dir = "mkdir " + cfg_dir
    subprocess.run(["mkdir", cfg_dir])
    subprocess.run(["touch", cfg_dir + "config"])

def display(arg):
    print(arg)


request()
display(local_folder)
display(remote_folder)
load_configs(local_folder)

#bash_command = "echo YAY!"
#os.system(bash_command)



########## How to pipe with subprocess.
# # print 'Hello World' to stdout
# command1 = ['echo']
# command1.append('Hello World')
# process1 = subprocess.Popen(command1,stdout=subprocess.PIPE)
#
# # Find 'hello' in the input and print that match to stdout
# command2 = ['grep']
# command2.append('-o')
# command2.append('-i')
# command2.append('hello')
# process2 = subprocess.Popen(command2,stdin=process1.stdout,stdout=subprocess.PIPE)
