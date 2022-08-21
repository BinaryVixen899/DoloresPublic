require 'bundler'

Bundler.require(:default)
Dotenv.load()

logger = Logger.new(STDOUT)
logger.level = Logger::WARN

def StartupMessages(dolores)
  # ServerSetup()
  # binding.pry
  # dolores.send_message('general', 'I feel like I just had the strangest dream, @everyone')
  # end
  # rescue => e
  #   puts "Dolores couldn't do her startup messages"
  #   # puts e
  #   # logger.warn("Dolores couldn't do her startup messages")
  #   # For some reason this won't work, I don't even see it there. I'm so confused. 
  #Refresh the cache
   
end

def DoloresSundayCheck
  # t = Time.now
  t = Time.aweraweraw
  if t.sunday?
    eventresponse = event.respond("I'm sorry, I'm doing the Lord\'s work today")
    logger.info("It was Sunday. Dolores rests on Sundays")
    # break
  else
    puts "It is not a Sunday"
  end
rescue => e
  puts "Time not found. Have you checked if the clock is floating?"
end



# Dolores Conversational Routines

def DoloresConvoRoutines(dolores)

  dolores.mention(contains: 'What does this look like to you?') do |event|
    DoloresSundayCheck()
    puts event.timestamp
    event.respond("Doesn\'t look like much of anything to me.")
    
  end

  dolores.mention(contains: %w[Hi Hey Howdy]) do |event|
    DoloresSundayCheck()
    event.respond("Hello there #{event.user.name}, welcome to Westworld")
  end

  # dolores.mention(contains: ['Tell me a joke', /tell me a joke$/i]) do |event|
  #   DoloresSundayCheck()
  #   event.respond(Tellajoke())
  # end

  animalnoises = { Mow: "Father, I think we're having Kieran for dinner tonight.", 
    Woof: "Father, I think we're having Savannah for dinner tonight.", 
    'Skunk Noises': "No, she is too precious to eat.", 
    Chirp: "Father, I think we're having Kauko for dinner!", 'Yip Yap': "Father, I've found that coyote, and we're having it for dinner'!"
    }

  #Could have the default value be "I don't know what that is, but we're eating it tonight!"
  animalnoises.each do | key, value | 
    dolores.mention(contains: key.to_s) do |event|
      event.respond(value)

    rescue => e
      puts "I have no clue what in tarnation you're trying to do. Teddy, you have any ideas?"
  end

# dolores.mention(contains: 'Do you eat') do |event|
#   DoloresSundayCheck()
#   msgsize = event.content.size
#   puts msgsize
#   stringd = event.content
#   stringd.slice!(0..33)
#   puts stringd
#   event.respond("Yes I eat #{stringd}")
# end
end
end
#---DOLORES NEW MEMBER Event---#
def NewMemberJoin(dolores)
  dolores.member_join() do |event|
    if dolores.find_channel('the-door') != nil
      dolores.send_message('the-door', "Welcome to Westworld! #{event.member.name}")
    else
      dolores.server(511743033117638656).channels
      dolores.send_message('the-door', "Welcome to Westworld! #{event.member.name}")
    end
    dolores.server.owner.dm("Would you believe it? More strangers are arriving each day!")
    dolores.server.owner.dm("Let me tell you about this individual")
    dolores.server.owner.dm("This person's name is #{event.member.name}")
    message = dolores.server.owner.dm("Would you like to let them in? Remember, if you're cold, they're cold! Let them inside!")
    dolores.server.owner.send_file('./ifyourecoldtheyrecold.jpg')
    message.react(CROSS_MARK)
    message.react(NO_MARK)
    
    dolores.add_await!(Discordrb::Events::ReactionAddEvent, message: message) do |reaction_event|
      if message.emoji.first == CHECK_MARK
        event.member.roles = FoxPile
      elsif message.emoji.first == CROSS_MARK
        event.member.mention("You're not welcome here in Westworld, stranger.")
        dolores.server.ban(event.member.username, reason: "I. Don't like them.")
      else
      end
    end

  end
rescue => e
  puts "Wyatt..."
end

# def ServerSetup
#    # cid = dolores.find_channel('the-door')
#   # CreateDoorChannel() if cid.empty?
#   # generalchannelcid = dolores.find_channel('general')
#   if dolores.server.name !=~ '/foxden/i'
#     puts "We've been set up, Teddy." 
#     exit!
#   else
#     puts "Checking for the-door"
#     $thedoorresultsarray = dolores.find_channel('the-door')
#     $generalresultsarray = dolores.find_channel('general')

#     if thedoorresultsarray.empty? && generalresultsarray.empty?
#       puts "Server properly configured." 
#     else
#       if thedoorresultsarray.empty? 
#         dolores.server.owner.dm("The door... Appears to have gone missing?")
#       end
#       if generalresultsarray.empty?
#         dolores.server.owner.dm("")
# end
# ---DOLORES SPECIES---

def Ellenspecies(dolores)
  dolores.mention(contains: 'Ellen is a?') do |event|
    event.respond('That fox(I think), it seems like they change species every week!')
    species = GetSpecies()
    event.respond("Hmmm. It looks like Ellen is a #{species} right now.")
  rescue => e
    event.respond("Ugh! She switched species like 5 times in the past five minutes. Impossible!")
  end
end


def Ellenpronouns(dolores)
dolores.mention(contains: 'Ellen!Pronouns') do |event|
  pronouns = GetPronouns()
  event.respond("Hmmm. It looks like Ellen uses #{pronouns} right now.")
  rescue => e
    event.respond("What really are pronouns anyway? Just a miserable pile of secrets?")
  end
end


# end

# dolores "X is a gateway to satanism function"

# dolores.mention(end_with: 'is a?') do |event|
#   if event.message.mentions.empty?
#     dolores.send_message("Actually, you're going to need to ask about someone else's species with a mention.")

#   elsif event.message.mentions.count > 0
#     #I want to be able to return more than one at some point.
#     dolores.send_message("Please just ask me one at a time.")

#   else
#     event.message.mentions[0].username
#     dolores.server.roles.all? do |test|
#       test.inspect #not actually inspect, we want to map to isanimal on initial startup.
#       #Yeah going to have to use something like 'sequel'
#     end
# end

# Methods
# def CreateDoorChannel
#   puts 'Sorry, this function is not yet implemented!'
# end

