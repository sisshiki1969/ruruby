# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.73 s | 2.74 s | x 1.58 |
| app_mandelbrot.rb | 2.14 s | 6.16 s | x 2.88 |
| fibo.rb | 0.5 s | 2.28 s | x 4.56 |
| block.rb | 0.39 s | 0.98 s | x 2.51 |
| ao_bench.rb | 9.25 s | 25.09 s | x 2.71 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 14.19  K | 9.12  K | x 0.64 |
| app_mandelbrot.rb | 0.01  M | 1.77  M | x 124.30 |
| fibo.rb | 13.95  K | 4.60  K | x 0.33 |
| block.rb | 14.00  K | 4.57  K | x 0.33 |
| ao_bench.rb | 0.01  M | 4.52  M | x 306.88 |
