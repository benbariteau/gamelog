# gamelog
A webapp for tracking video game progress

# Setup
The only setup currently required is to initialize the database with the table schemas. Easiest way to do this (with sqlite3 installed) is to run this in the project root:
```
$ cat schema/* | sqlite3 gamelog.db
```
