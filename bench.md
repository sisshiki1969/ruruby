# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.76 s | 2.73 s | x 1.55 |
| app_mandelbrot.rb | 2.26 s | 6.85 s | x 3.03 |
| fibo.rb | 0.48 s | 2.36 s | x 4.92 |
| block.rb | 0.38 s | 1.04 s | x 2.74 |
| ao_bench.rb | 9.41 s | 27.34 s | x 2.91 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 14.18  K | 11.65  K | x 0.82 |
| app_mandelbrot.rb | 0.01  M | 1.76  M | x 124.67 |
| fibo.rb | 13.97  K | 4.56  K | x 0.33 |
| block.rb | 14.05  K | 4.56  K | x 0.32 |
| ao_bench.rb | 0.01  M | 4.50  M | x 307.59 |
