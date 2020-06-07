# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.29 ± 0.01 s | 2.45 ± 0.01 s | x 1.89 |
| app_mandelbrot.rb | 1.90 ± 0.01 s | 8.15 ± 0.07 s | x 4.28 |
| fibo.rb | 0.48 ± 0.00 s | 2.55 ± 0.01 s | x 5.27 |
| block.rb | 0.42 ± 0.00 s | 0.89 ± 0.02 s | x 2.12 |
| ao_bench.rb | 7.51 ± 0.02 s | 33.81 ± 0.39 s | x 4.50 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.4K | 2.3K | x 0.20 |
| app_mandelbrot.rb | 11.3K | 3.4K | x 0.30 |
| fibo.rb | 11.3K | 2.1K | x 0.18 |
| block.rb | 11.1K | 2.0K | x 0.18 |
| ao_bench.rb | 11.3K | 9.2K | x 0.81 |
