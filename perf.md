# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.84 s | 2.97 s | x 1.61 |
| app_mandelbrot.rb | 2.21 s | 7.01 s | x 3.17 |
| fibo.rb | 0.48 s | 2.35 s | x 4.90 |
| block.rb | 0.39 s | 1.0 s | x 2.56 |
| ao_bench.rb | 9.86 s | 28.5 s | x 2.89 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 14.24  K | 11.56  K | x 0.81 |
| app_mandelbrot.rb | 0.01  M | 1.76  M | x 124.72 |
| fibo.rb | 13.94  K | 4.56  K | x 0.33 |
| block.rb | 14.04  K | 4.50  K | x 0.32 |
| ao_bench.rb | 0.01  M | 4.50  M | x 303.76 |
