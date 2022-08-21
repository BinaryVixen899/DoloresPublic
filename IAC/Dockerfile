FROM ruby:3.0 AS setup
# RUN apt-get update -qq && apt-get install -y
RUN mkdir /prod/
COPY *.rb /prod/
COPY Gemfile /prod/Gemfile
COPY Gemfile.lock /prod/Gemfile.lock 
WORKDIR /prod/
RUN bundle install 
RUN chmod +x /prod/*.rb
EXPOSE 443
CMD ["ruby", "./dolores.rb"]