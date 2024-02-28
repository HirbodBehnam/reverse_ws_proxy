# Reverse TCP over Websocket Proxy

A TCP over Websocket port forwarder that the server sends the SYN packet.

## What?
A TCP port forwarder which wraps each TCP connection in a websocket connection and the destination server establishes the websocket connection. Mostly, this program works like [ReverseTCPProxy](https://github.com/HirbodBehnam/ReverseTCPProxy) but the only difference is the fact that the every connection even the controller is established from the destination to client in websocket protocol. It's also worth noting that because of websocket, the local server can be put behind Cloudflare.

### How?
At first let us assume that there are two applications:
1. Local: It wants to establish connections to a service of Remote server.
2. Remote: Serves an service which the Local client what's to connect to.
However, for any reason, you want the Remote server to send the SYN packet to Local client. So, the Remote server established a websocket connection with Local client called "Control". In this stream, the local sends an UUID for each incoming TCP connection. Then the server opens a websocket connection for each TCP connection and sends the UUID as the first packet. The client then forwards each packet of the TCP connection into the corresponding websocket stream.

## Running
### Building
It should be possible to just build this application with `cargo build --release`. No more configuration is needed.

### Local Client
For the local client, you need to provide two arguments to the program:
* `tcp_listen_address`: The TCP address that we should bind and accept connections. These connections are intended to go to the Remote server.
* `cloudflare_listen_address`: Despite its name that has cloudflare in it, it's just the address that expects the Remote server to connect to the Local client.

### Remote Server
Remote server also expects two arguments:
* `cloudflare_server_address`: The address which the Local client is reachable. This should be like this format: `ws://your.domain:12345`. Note that it should not contain a leading `/`. For example `ws://your.domain:12345/` is wrong.
* `forward_address`: Where should the TCP streams be forwarded?