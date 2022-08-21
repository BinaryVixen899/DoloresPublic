# require 'discordrb'
# require 'net/http'
# # require 'nokogiri'
# require 'restclient'
# # require 'sequel'
# # require 'sqlite3'
# require 'dotenv/load'
# require 'logger' 
require 'date'
require 'bundler'
Bundler.require(:default)

#From Dolores 
require_relative 'dolorespluralkit'
require_relative 'doloresnotion'
require_relative 'doloresmessages'


# Variablesri
# DEBUG = 'FALSE'
CHECK_MARK = "\u2713"
CROSS_MARK = "\u274c"
VERSION = 1.0

# DB = Sequel.sqlite
# DB.create
# I do not need a DB
dolores = Discordrb::Bot.new token: ENV['TOKEN']

#Declare logger
logger = Logger.new(STDOUT)
logger.level = Logger::WARN

# begin
#   doloresav = File.open('./dolores.jpg')
#   file_data = doloresav.read
#   doloresav.close
#   puts "Sucessfully Loaded Avatar"
# rescue StandardError => e
#   puts "Something feels wrong, I don't. I don't know what I look like."
#   puts e.to_s
# end

#Dolores Startup
dolores.ready do |event|
  puts 'test'
  dolores.name='Dolores Abernathy'
  logger.info("Dolores' name #{dolores.name}")
  # dolores.game = 'Westworld'
  # dolores.watching = 'The World Burning'
  doloresmusic = dolores.listening = 'Wicked Games (Ramin Djawadi)'
  logger.info("Dolores' music: #{doloresmusic}")
  #TODO: Have her randomly choose between these 
  # dolores.profile.avatar='https://upload.wikimedia.org/wikipedia/en/6/64/DoloresAbernathy.png'
  #Just call a method here to do the setup
  # TODO: Read this in
  dolores.server(REDACTED).members
  dolores.server(REDACTED).channels
  $generalchannel = dolores.find_channel('general')
  #Reimplement the date functionality here 
  t = Time.now
  if t.wednesday?
    dolores.send_message("#{$generalchannel}", "I feel like I just had the strangest dream...")
  end
  
  logger.debug("Dolores has started up sucessfully!")
  logger.debug("Dolores Version: #{VERSION}")

  
end


DoloresConvoRoutines(dolores)
NewMemberJoin(dolores)
Ellenpronouns(dolores)
Ellenspecies(dolores)
#If we load stuff here we should be able to do This


# TO DOS
# I really, really want to make this multithreaded
  
dolores.run

