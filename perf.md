# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.91 s | 3.22 s | x 1.69 |
| app_mandelbrot.rb | 2.36 s | 7.46 s | x 3.16 |
| fibo.rb | 0.51 s | 2.41 s | x 4.73 |
| block.rb | 0.41 s | 1.14 s | x 2.78 |
| ao_bench.rb | 10.19 s | 28.2 s | x 2.77 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 14.10  K | 11.51  K | x 0.82 |
| app_mandelbrot.rb | 0.01  M | 1.76  M | x 123.68 |
| fibo.rb | 14.05  K | 4.74  K | x 0.34 |
| block.rb | 13.90  K | 4.52  K | x 0.33 |
| ao_bench.rb | 0.01  M | 4.50  M | x 304.09 |
