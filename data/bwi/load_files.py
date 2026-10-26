#! /usr/bin/env python3
'''
put into ./public_maisn, the folder holding the data
cd into the folder and start with 
`python3 -m load_files

the files to load are stored in the shelf 'files_to_input' in the same dir
to restart the load process from scratch on a clean database delete that file
so that it gets recreated

we use psql with --single-transaction option, so as to be able to react on errors.
when a file has been loaded successfully it gets removed from the shelf and won't 
be loaded at the next try. 
'''


import shelve
from pathlib import Path
import subprocess

here = Path.cwd()
if not (here / 'files_to_input').exists():
    with shelve.open(here / 'files_to_input', 'c') as shelf:
        files = list(here.glob('*.sql'))        
        shelf['files'] = files
        shelf['done'] = 0
        shelf['to_do'] = len(files)
        print(f"created new file list with {len(files)} files")


with shelve.open('files_to_input', 'c') as shelf:
    files = shelf['files']
    files_to_do = list(files)
    done = shelf['done']
    todo = shelf['to_do']
    comm = 'psql -X --set ON_ERROR_STOP=on --single-transaction bwi < '
    for file in files_to_do:
        print(f'\n starting {file}')
        proc = subprocess.run([comm + str(file)],
                              capture_output=True, shell=True)
        if proc.returncode != 0:
            # if something went wrong print the error and stop the loop
            print(proc.stderr)
            break
        else:
            done += 1
            print(f' loaded {file} \n {done} of todo {todo} \n')
            files.remove(file)


    # store the new files, lacking the succesfully done files
    shelf['files'] = files
    shelf['done'] = done


# debugging code to see what is in files
# for file in files :
#    print(f"{file}")'


