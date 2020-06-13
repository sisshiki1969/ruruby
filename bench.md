# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.5  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.30 ± 0.02 s | 2.55 ± 0.10 s | x 1.96 |
| app_mandelbrot.rb | 2.01 ± 0.03 s | 8.34 ± 0.05 s | x 4.16 |
| fibo.rb | 0.49 ± 0.01 s | 2.26 ± 0.02 s | x 4.60 |
| block.rb | 0.40 ± 0.01 s | 0.91 ± 0.02 s | x 2.29 |
| ao_bench.rb | 7.64 ± 0.07 s | 33.23 ± 0.38 s | x 4.35 |
| optcarrot | 43.79 ± 4.32 fps | 8.54 ± 0.18 fps | x 5.13 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.3K | 2.4K | x 0.21 |
| app_mandelbrot.rb | 11.3K | 47.3K | x 4.19 |
| fibo.rb | 11.0K | 2.1K | x 0.19 |
| block.rb | 11.3K | 2.0K | x 0.18 |
| ao_bench.rb | 11.4K | 63.8K | x 5.58 |
