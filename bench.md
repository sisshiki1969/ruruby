# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.82 ± 0.13 s | 2.82 ± 0.26 s  | x 1.55 |
| app_mandelbrot.rb | 2.29 ± 0.02 s | 6.15 ± 0.03 s  | x 2.69 |
|      fibo.rb      | 0.48 ± 0.01 s | 1.93 ± 0.04 s  | x 3.98 |
|     block.rb      | 0.38 ± 0.01 s | 1.03 ± 0.00 s  | x 2.70 |
|    ao_bench.rb    | 9.28 ± 0.67 s | 26.77 ± 0.80 s | x 2.89 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 45.10 ± 0.26 fps  | 8.07 ± 0.14 fps  | x 5.59 |
| optcarrot --opt | 140.44 ± 6.36 fps | 32.03 ± 6.53 fps | x 4.38 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.1K |  4.7K  | x 0.33  |
| app_mandelbrot.rb | 14.1K |  6.5K  | x 0.46  |
|      fibo.rb      | 13.8K |  4.6K  | x 0.34  |
|     block.rb      | 13.8K |  4.6K  | x 0.34  |
|    ao_bench.rb    | 14.5K |  7.2K  | x 0.50  |
|     optcarrot     | 68.4K | 81.1K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 16.87 |
