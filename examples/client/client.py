import sys
import zmq

ctx = zmq.Context()
req = ctx.socket(zmq.REQ)
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

        resp = req.recv_string()
        print('RESP:', resp)
    except Exception as e:
        print(e)
        quit(666)
