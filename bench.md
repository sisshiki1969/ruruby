# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.83 ± 0.06 s | 2.63 ± 0.02 s | x 1.44 |
| app_mandelbrot.rb | 2.28 ± 0.01 s | 7.07 ± 0.19 s | x 3.11 |
| fibo.rb | 0.47 ± 0.01 s | 2.18 ± 0.04 s | x 4.67 |
| block.rb | 0.40 ± 0.01 s | 1.07 ± 0.03 s | x 2.65 |
| ao_bench.rb | 9.23 ± 0.06 s | 27.37 ± 0.49 s | x 2.96 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.8K | 4.8K | x 0.35 |
| app_mandelbrot.rb | 13.9K | 17.3K | x 1.25 |
| fibo.rb | 13.6K | 4.6K | x 0.34 |
| block.rb | 13.6K | 4.6K | x 0.34 |
| ao_bench.rb | 14.4K | 19.5K | x 1.36 |
