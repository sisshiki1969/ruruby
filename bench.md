# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.5  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.29 ± 0.03 s | 2.58 ± 0.09 s | x 1.99 |
| app_mandelbrot.rb | 1.89 ± 0.02 s | 8.45 ± 0.60 s | x 4.47 |
| fibo.rb | 0.53 ± 0.02 s | 2.18 ± 0.01 s | x 4.09 |
| block.rb | 0.40 ± 0.01 s | 0.94 ± 0.05 s | x 2.34 |
| ao_bench.rb | 7.77 ± 0.24 s | 31.35 ± 0.63 s | x 4.03 |
| optcarrot | 48.48 ± 0.38 fps | 8.96 ± 0.09 fps | x 5.41 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.2K | 2.3K | x 0.21 |
| app_mandelbrot.rb | 11.3K | 46.5K | x 4.14 |
| fibo.rb | 11.5K | 2.1K | x 0.18 |
| block.rb | 11.0K | 2.0K | x 0.18 |
| ao_bench.rb | 11.7K | 60.2K | x 5.16 |
| optcarrot | 75.8K | 94.0K | x 1.24 |
