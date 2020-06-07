# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.34 ± 0.01 s | 2.62 ± 0.02 s | x 1.96 |
| app_mandelbrot.rb | 1.98 ± 0.02 s | 8.78 ± 0.22 s | x 4.43 |
| fibo.rb | 0.56 ± 0.07 s | 2.54 ± 0.02 s | x 4.52 |
| block.rb | 0.44 ± 0.01 s | 0.92 ± 0.02 s | x 2.10 |
| ao_bench.rb | 7.79 ± 0.04 s | 34.41 ± 0.52 s | x 4.42 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.3K | 2.4K | x 0.21 |
| app_mandelbrot.rb | 11.3K | 5.3K | x 0.47 |
| fibo.rb | 11.8K | 2.1K | x 0.18 |
| block.rb | 11.2K | 2.0K | x 0.18 |
| ao_bench.rb | 11.6K | 11.4K | x 0.99 |
