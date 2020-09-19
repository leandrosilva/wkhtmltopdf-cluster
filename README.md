# WkHTMLtoPDF Cluster

This is a side project to experiment in creating a cluster of processes to convert `HTML` into `PDF` backed by `wkhtmltopdf` through its C API `libwkhtmltox`.

## Build

    $ cargo build -p wkhtmltopdf-cluster-worker --release
    $ cargo build -p wkhtmltopdf-cluster-manager --release

## Run

    $ ./target/release/wkhtmltopdf-cluster-manager start -i 3 -o ./examples/pdf

## Copyright

Leandro Silva <<leandrodoze@gmail.com>>
