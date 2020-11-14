import sys
import zmq
import os

id = f'C{os.getpid()}'
print('ID:', id)

context = zmq.Context()
req = context.socket(zmq.REQ)
req.identity = id.encode('ascii')
req.setsockopt(zmq.SNDTIMEO, 10000)
req.setsockopt(zmq.RCVTIMEO, 10000)
req.connect('tcp://127.0.0.1:6660')

# msg = 'https://www.google.com.br'
msg = 'file:///D:/Projects/GitHub/wkhtmltopdf-cluster/examples/client/sample1.html'
# msg = 'file:///Users/leandro/Projects/rust/wkhtmltopdf-cluster/examples/client/sample1.html'

if len(sys.argv) > 1:
    msg = sys.argv[1]

for i in range(1, 11):
    print(">>", i)
    try:
        req.send_string(msg)
        print('REQ:', msg)

        resp = req.recv_multipart()
        print('RESP:', resp)
    except Exception as e:
        print(e)
        quit(666)
