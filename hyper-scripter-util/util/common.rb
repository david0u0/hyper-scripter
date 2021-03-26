class HSEnv
  def initialize(script_dir)
    find_hs_env(script_dir)
    puts "hyper script home = #{@hs_home}, executable = #{@hs_exe}"
  end

  def find_hs_env(script_dir)
    path_script = File.join(script_dir, 'hs_env.sh')
    env = `bash #{path_script}`.delete("\n")
    @hs_home, @hs_exe = env.split(':')
  end

  def home
    @hs_home
  end

  def do_hs(arg, all = true, path = @hs_home)
    cmd = hs_command_str(arg, all, path)
    `#{cmd}`
  end

  def exec_hs(arg, all = true, path = @hs_home)
    cmd = hs_command_str(arg, all, path)
    exec "#{cmd}"
  end

  private
  def hs_command_str(arg, all, path)
    tags_str = ''
    if all
      tags_str = "-f all"
    end
    "#{@hs_exe} --no-alias --timeless -H #{path} #{tags_str} #{arg}"
  end

end

DIR = File.dirname(__FILE__)
HS_ENV = HSEnv.new(DIR)
