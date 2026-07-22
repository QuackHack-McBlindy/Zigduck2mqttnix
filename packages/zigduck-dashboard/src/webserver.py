import http.server
import socketserver
import os
import urllib.parse
import json
import uuid
import time
import ssl
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("--password-file", required=True)
parser.add_argument("--port", type=int, default=13337)
parser.add_argument("--cert-file", default="")
parser.add_argument("--key-file", default="")
parser.add_argument("--workdir", default=os.getcwd())
args = parser.parse_args()

with open(args.password_file, "r") as f:
    PASSWORD = f.read().strip()

sessions = {}

# Store the workdir in a global variable
WORKDIR = args.workdir

class SimpleAuthHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *pos_args, **kwargs):
        # Use the global WORKDIR
        super().__init__(*pos_args, directory=WORKDIR, **kwargs)

    def do_GET(self):
        auth_cookie = self.headers.get('Cookie', "")
        is_authenticated = False
        for cookie in auth_cookie.split(';'):
            cookie = cookie.strip()
            if cookie.startswith('auth_token='):
                token = cookie.split('auth_token=')[1]
                if token in sessions:
                    is_authenticated = True
        
        if self.path in ['/login', '/login.html', '/submit']:
            return super().do_GET()
        
        if not is_authenticated:
            self.send_response(302)
            self.send_header('Location', '/login.html')
            self.end_headers()
            return
        
        return super().do_GET()
    
    def do_POST(self):
        if self.path == '/submit':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length).decode('utf-8')
            parsed_data = urllib.parse.parse_qs(post_data)
            password = parsed_data.get('password', [""])[0]
            
            if password == PASSWORD:
                token = str(uuid.uuid4())
                sessions[token] = time.time()
                
                self.send_response(302)
                self.send_header('Location', '/')
                self.send_header('Set-Cookie', f'auth_token={token}; Path=/; HttpOnly; SameSite=Lax')
                self.send_header('Set-Cookie', f'api_password={PASSWORD}; Path=/; SameSite=Lax')
                self.end_headers()
            else:
                self.send_response(401)
                self.send_header('Content-type', 'text/html')
                self.end_headers()
                self.wfile.write(b'<html><body>Access denied. <a href="/login.html">Try again</a></body></html>')
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass

if __name__ == '__main__':
    os.chdir(args.workdir)   # Also change the process' current directory (optional)
    httpd = socketserver.TCPServer(("", args.port), SimpleAuthHandler)

    if args.cert_file and args.key_file and os.path.exists(args.cert_file) and os.path.exists(args.key_file):
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(args.cert_file, args.key_file)
        httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)
        print(f"🦆 HTTPS on port {args.port}")
    else:
        print(f"🦆 HTTP on port {args.port}")

    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        httpd.shutdown()
