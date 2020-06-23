# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.71 ± 0.02 s | 2.38 ± 0.02 s  | x 1.39 |
| app_mandelbrot.rb | 2.16 ± 0.02 s | 5.75 ± 0.17 s  | x 2.67 |
|      fibo.rb      | 0.46 ± 0.01 s | 1.88 ± 0.01 s  | x 4.10 |
|     block.rb      | 0.37 ± 0.01 s | 0.94 ± 0.02 s  | x 2.58 |
|    ao_bench.rb    | 8.94 ± 0.08 s | 25.35 ± 0.14 s | x 2.84 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 48.98 ± 0.78 fps  | 8.68 ± 0.20 fps  | x 5.64 |
| optcarrot --opt | 146.36 ± 4.71 fps | 38.20 ± 0.43 fps | x 3.83 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.1K |  4.7K  | x 0.34  |
| app_mandelbrot.rb | 14.1K |  6.5K  | x 0.46  |
|      fibo.rb      | 13.8K |  4.6K  | x 0.33  |
|     block.rb      | 13.8K |  4.6K  | x 0.33  |
|    ao_bench.rb    | 14.5K |  7.2K  | x 0.49  |
|     optcarrot     | 68.4K | 81.3K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 17.90 |
