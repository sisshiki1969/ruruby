# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.78 s | 3.09 s | x 1.74 |
| app_mandelbrot.rb | 2.25 s | 8.24 s | x 3.66 |
| fibo.rb | 0.49 s | 2.86 s | x 5.84 |
| block.rb | 0.42 s | 1.07 s | x 2.55 |
| ao_bench.rb | 9.36 s | 30.31 s | x 3.24 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.91  K | 11.59  K | x 0.83 |
| app_mandelbrot.rb | 0.01  M | 1.76  M | x 127.30 |
| fibo.rb | 13.77  K | 4.62  K | x 0.34 |
| block.rb | 13.65  K | 4.55  K | x 0.33 |
| ao_bench.rb | 0.01  M | 4.50  M | x 310.15 |
