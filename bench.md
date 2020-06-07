# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.34 ± 0.01 s | 2.62 ± 0.13 s | x 1.96 |
| app_mandelbrot.rb | 1.99 ± 0.01 s | 8.46 ± 0.07 s | x 4.26 |
| fibo.rb | 0.50 ± 0.01 s | 2.68 ± 0.12 s | x 5.41 |
| block.rb | 0.43 ± 0.01 s | 0.98 ± 0.03 s | x 2.26 |
| ao_bench.rb | 8.04 ± 0.48 s | 35.98 ± 1.71 s | x 4.47 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.2K | 2.3K | x 0.20 |
| app_mandelbrot.rb | 11.2K | 3.8K | x 0.34 |
| fibo.rb | 11.1K | 2.1K | x 0.18 |
| block.rb | 11.1K | 2.0K | x 0.18 |
| ao_bench.rb | 12.1K | 9.3K | x 0.77 |
