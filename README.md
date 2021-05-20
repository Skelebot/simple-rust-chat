## simple-rust-chat

A simple text chat server and client written in rust in under 300 lines of code, with no dependencies (except anyhow (I'm sorry it's just too good))

**The server**:
 - waits for new connections
 - identifies users by their requested nickname, does not allow duplicate nicknames
 - spawns a new listening thread for every new user
 - spawns a "broadcast" thread that sends received messages to every connected client
 - uses a multi-producer single-consumer channel to send client messages to the broadcast thread
 - handles users connecting and disconnecting
 - logs everything to stdout

**The client**:
 - connects to a server and transmits the requested nickname
 - sends lines read from stdin to the server
 - prints lines received from the server to stdout
