# WkHTMLtoPDF Cluster

This is a side project to experiment in creating a cluster of processes to convert `HTML` into `PDF` backed by `wkhtmltopdf` through its C API `libwkhtmltox`.

## Build

    $ cargo build --release

## Run

Spin up 3 workers:

    $ target/release/broker start -i 3 -o ./examples/pdf/

Then test it with a client:

    $ cd ./examples/client
    $ source venv/bin/activate
    $ python client.py

## Copyright

Leandro Silva <<leandrodoze@gmail.com>>
