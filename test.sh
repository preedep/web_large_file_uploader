curl -v -X POST -H "Content-Type: application/json" \
 -d '{"file_name":"test.csv","file_size":102400,"file_hash":"X1234BC"}' http://localhost:8888/api/v1/start_upload
