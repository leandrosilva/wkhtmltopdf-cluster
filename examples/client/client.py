import zmq

ctx = zmq.Context()
req = ctx.socket(zmq.REQ)
req.connect('tcp://127.0.0.1:6660')
req.send_string('https://www.google.com.br')
resp = req.recv_string()
print('RESP:', resp)
