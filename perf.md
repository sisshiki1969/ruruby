# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.74 s | 2.92 s | x 1.68 |
| app_mandelbrot.rb | 2.34 s | 7.8 s | x 3.33 |
| fibo.rb | 0.54 s | 2.86 s | x 5.30 |
| block.rb | 0.47 s | 1.07 s | x 2.28 |
| ao_bench.rb | 9.53 s | 30.68 s | x 3.22 |
Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.85  K | 11.62  K | x 0.84 |
| app_mandelbrot.rb | 0.01  M | 1.76  M | x 127.02 |
| fibo.rb | 13.56  K | 4.64  K | x 0.34 |
| block.rb | 13.68  K | 4.62  K | x 0.34 |
| ao_bench.rb | 0.01  M | 4.50  M | x 312.82 |
