export AFL_DISABLE_TRIM=1
export AFL_NO_UI=1
for i in {1..20}
do
  cargo afl fuzz -i ../data/ -o out -a binary -P explore -S fuzz$i  -p explore target/debug/fuzz-afl &
done
export AFL_DISABLE_TRIM=0;
for i in {21..60}
do
  cargo afl fuzz -i ../data/ -o out -a binary -P exploit -S fuzz$i  -p exploit target/debug/fuzz-afl &
done