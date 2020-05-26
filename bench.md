# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.32 s | 3.32 s | x 2.52 |
| app_mandelbrot.rb | 1.92 s | 8.23 s | x 4.29 |
| fibo.rb | 0.5 s | 2.48 s | x 4.96 |
| block.rb | 0.44 s | 0.89 s | x 2.02 |
| ao_bench.rb | 7.59 s | 34.71 s | x 4.57 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.76  K | 8.59  K | x 0.73 |
| app_mandelbrot.rb | 0.01  M | 1.78  M | x 152.76 |
| fibo.rb | 10.79  K | 2.06  K | x 0.19 |
| block.rb | 10.97  K | 2.01  K | x 0.18 |
| ao_bench.rb | 0.01  M | 4.55  M | x 388.01 |
