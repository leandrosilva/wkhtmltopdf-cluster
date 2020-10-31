import sys
import zmq

ctx = zmq.Context()
req = ctx.socket(zmq.REQ)
req.setsockopt(zmq.RCVTIMEO, 3000)
req.connect('tcp://127.0.0.1:6660')

msg = 'https://www.google.com.br'
if len(sys.argv) > 1:
    msg = sys.argv[1]

try:
    req.send_string(msg)
    print('REQ:', msg)

    resp = req.recv_string()
    print('RESP:', resp)
except Exception as e:
    print(e)
    quit(666)
