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

msg = """{
    "url": "file:///D:/Projects/GitHub/wkhtmltopdf-cluster/examples/client/sample1.html",
    "global": {
        "documentTitle": "WkHTMLtoPDF Cluster :: Example Client",
        "copies": 2,
        "size.pageSize": "A5"
    },
    "object": {
        "load.debugJavascript": true,
        "load.windowStatus": "ready"
    },
    "onWarning": {
        "action": "abort",
        "triggerWords": ["error", "fail"]
    }
}"""

if len(sys.argv) > 1:
    msg = sys.argv[1]

for i in range(1, 2):
    print(">>", i)
    try:
        req.send_string(msg)
        print('REQ:', msg)

        resp = req.recv_multipart()
        print('RESP:', resp)
    except Exception as e:
        print(e)
        quit(666)
