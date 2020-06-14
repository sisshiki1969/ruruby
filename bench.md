# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |       ruby       |     ruruby      |  rate  |
| :---------------: | :--------------: | :-------------: | :----: |
| so_mandelbrot.rb  |  1.77 ± 0.07 s   |  2.82 ± 0.10 s  | x 1.60 |
| app_mandelbrot.rb |  2.21 ± 0.06 s   |  5.85 ± 0.08 s  | x 2.64 |
|      fibo.rb      |  0.50 ± 0.02 s   |  1.94 ± 0.01 s  | x 3.90 |
|     block.rb      |  0.38 ± 0.01 s   |  1.02 ± 0.02 s  | x 2.66 |
|    ao_bench.rb    |  9.36 ± 0.47 s   | 27.80 ± 4.02 s  | x 2.97 |
|     optcarrot     | 47.61 ± 2.23 fps | 8.81 ± 0.22 fps | x 5.40 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 14.2K |  4.9K  | x 0.34 |
| app_mandelbrot.rb | 14.2K | 17.3K  | x 1.22 |
|      fibo.rb      | 14.0K |  4.5K  | x 0.32 |
|     block.rb      | 14.0K |  4.6K  | x 0.33 |
|    ao_bench.rb    | 14.7K | 22.6K  | x 1.54 |
