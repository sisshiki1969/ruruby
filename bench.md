# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.82 ± 0.09 s | 2.86 ± 0.10 s | x 1.57 |
| app_mandelbrot.rb | 2.31 ± 0.05 s | 6.24 ± 0.04 s | x 2.70 |
| fibo.rb | 0.49 ± 0.03 s | 2.04 ± 0.02 s | x 4.16 |
| block.rb | 0.38 ± 0.00 s | 1.07 ± 0.06 s | x 2.78 |
| ao_bench.rb | 10.45 ± 0.04 s | 27.44 ± 0.71 s | x 2.62 |
| optcarrot | 43.99 ± 0.23 fps | 8.06 ± 0.09 fps | x 5.46 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 14.2K | 4.9K | x 0.35 |
| app_mandelbrot.rb | 14.2K | 17.3K | x 1.22 |
| fibo.rb | 14.0K | 4.6K | x 0.33 |
| block.rb | 14.0K | 4.6K | x 0.33 |
| ao_bench.rb | 14.8K | 20.9K | x 1.41 |
