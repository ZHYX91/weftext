"""The actual owned loopback relay and one accepted-reply-loss injection."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from http.client import HTTPConnection
import socket
import threading
from urllib.parse import urlsplit

from _cli_contract_common import MAX_LOG_READ, digest, require, unique_json


class HostedRelay(ThreadingHTTPServer):
    """Owned loopback transport fault; never invent a product operation reply."""
    daemon_threads = False
    block_on_close = True

    def __init__(self, upstream):
        target = urlsplit(upstream)
        require(target.scheme == 'http' and target.hostname == '127.0.0.1'
                and target.path in ('', '/') and target.port is not None, 'Relay target is not owned loopback')
        self.upstream = (target.hostname, target.port)
        self.requests, self.errors = [], []
        self.drop_next_save = self.block_status = False
        self.accepted_loss = None
        self.socket_lock = threading.Lock()
        self.incoming, self.upstream_handles = set(), set()
        self.closing = False

        class Handler(BaseHTTPRequestHandler):
            protocol_version = 'HTTP/1.0'

            def log_message(self, *_args):
                pass  # Authorization headers and credentials are never logged.

            def do_GET(self):
                self.forward()

            def do_POST(self):
                self.forward()

            def forward(self):
                relay = self.server
                length = int(self.headers.get('Content-Length', '0'))
                if not 0 <= length <= 2 * 1024 * 1024:
                    self.send_error(413)
                    return
                body = self.rfile.read(length)
                entry = {'method': self.command, 'path': self.path,
                         'requestLength': len(body), 'requestSha256': digest(body), 'forwarded': False}
                relay.requests.append(entry)
                if self.path == '/api/hosted/operation' and relay.block_status:
                    selector = unique_json(body)
                    entry['operationId'], entry['resource'] = selector.get('operationId'), selector.get('resource')
                    if selector == {'operationId': relay.accepted_loss['operationId'],
                                    'resource': relay.accepted_loss['resource']}:
                        entry['relayFault'] = 'Withheld original-status response after accepted save'
                        self.send_response(503)
                        self.send_header('Content-Length', '0')
                        self.end_headers()
                        return
                connection, shutdown_handle = HTTPConnection(*relay.upstream, timeout=60), None
                try:
                    connection.connect()
                    shutdown_handle = connection.sock.dup()
                    with relay.socket_lock:
                        if relay.closing:
                            raise ConnectionAbortedError('Owned relay is closing')
                        relay.upstream_handles.add(shutdown_handle)
                    entry['forwarded'] = True
                    connection.request(self.command, self.path,
                        body=body if self.command == 'POST' else None,
                        headers={'Content-Type': 'application/json',
                                 'Authorization': self.headers.get('Authorization', '')})
                    response = connection.getresponse()
                    with response:
                        raw = response.read(MAX_LOG_READ + 1)
                        require(len(raw) <= MAX_LOG_READ, 'Relay reply exceeded its bounded read')
                        code = response.status
                        content_type = response.headers.get('Content-Type', 'application/octet-stream')
                    entry.update(status=code, responseLength=len(raw), responseSha256=digest(raw))
                    value = unique_json(raw) if raw and 'application/json' in content_type else None
                    if self.path == '/api/hosted/save':
                        entry['operationId'] = unique_json(body)['operationId']
                    if self.path == '/api/hosted/export/chunk' and code == 200:
                        entry['chunk'] = {key: value[key] for key in
                                          ('requestId', 'offset', 'length', 'total', 'sha256', 'done')}
                    if (self.path == '/api/hosted/save' and relay.drop_next_save
                            and code == 200 and value.get('status') == 'committed'):
                        relay.drop_next_save = False
                        relay.block_status = True
                        relay.accepted_loss = value
                        entry['relayFault'] = 'Dropped real accepted save response'
                        self.close_connection = True
                        try:
                            self.connection.shutdown(socket.SHUT_RDWR)
                        except OSError:
                            pass
                        return
                    self.send_response(code)
                    self.send_header('Content-Type', content_type)
                    self.send_header('Content-Length', str(len(raw)))
                    self.end_headers()
                    self.wfile.write(raw)
                except Exception as error:
                    relay.errors.append({'path': self.path, 'errorType': type(error).__name__})
                    self.close_connection = True
                finally:
                    connection.close()
                    if shutdown_handle is not None:
                        with relay.socket_lock:
                            relay.upstream_handles.discard(shutdown_handle)
                        shutdown_handle.close()

        super().__init__(('127.0.0.1', 0), Handler)

    def get_request(self):
        connection, address = super().get_request()
        connection.settimeout(65)
        with self.socket_lock:
            self.incoming.add(connection)
        return connection, address

    def close_request(self, connection):
        try:
            super().close_request(connection)
        finally:
            with self.socket_lock:
                self.incoming.discard(connection)

    def close_owned_connections(self):
        with self.socket_lock:
            self.closing = True
            owned = [*self.incoming, *self.upstream_handles]
        for connection in owned:
            try:
                connection.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            connection.close()
