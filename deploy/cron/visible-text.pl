#!/usr/bin/env perl
#
# Texte visible d'une page HTML, lu sur l'entrée standard, écrit normalisé sur
# la sortie standard. Sert d'empreinte à hemicycle-candidate-sources.sh : les
# nonces, jetons Cloudflare, chronométrages en commentaire, attributs de style
# tirés au hasard et noms de fichiers de build changent à chaque requête sans
# rien changer au texte lu par un visiteur.
#
# Même découpage que visibleHtmlText() dans scripts/check-candidate-sources.mjs :
# commentaires, <head>, <script>, <style>, <noscript>, <template> et <svg>
# retirés, entités décodées, espaces resserrées. Une balise de bloc coupe le
# texte en fragments, une balise de texte (<a>, <strong>…) non.
#
# Les fragments sortent triés et dédoublonnés, un par ligne : certains sites
# mélangent et répètent à chaque requête un bandeau défilant de propositions.
# Un texte ajouté, retiré ou modifié change l'empreinte ; un simple changement
# d'ordre des blocs, non. Les extraits publiés sont cités mot pour mot, leur
# position dans la page ne compte pas.
#
# Seulement des fonctions intégrées à Perl (paquet perl-base).

use strict;
use warnings;

local $/;
my $html = <STDIN> // '';

$html =~ s/<!--.*?-->/ /gs;
$html =~ s/<(head|script|style|noscript|template|svg)\b.*?<\/\1\s*>/ /gsi;
# Un retour à la ligne du source HTML n'est pas une coupure de bloc.
$html =~ s/\s+/ /g;
my $inline = qr/a|abbr|b|bdi|bdo|br|cite|code|data|dfn|em|font|i|kbd|mark|q|s|samp|small|span|strong|sub|sup|time|u|var|wbr/i;
$html =~ s/<\/?(?:$inline)\b[^>]*>/ /gs;
$html =~ s/<[^>]*>/\n/gs;

my %named = (
    amp => '&', lt => '<', gt => '>', quot => '"', apos => "'",
    euro => "\x{20AC}", oelig => "\x{153}", OElig => "\x{152}",
    rsquo => "\x{2019}", lsquo => "\x{2018}", rdquo => "\x{201D}",
    ldquo => "\x{201C}", hellip => "\x{2026}", ndash => "\x{2013}",
    mdash => "\x{2014}", bull => "\x{2022}",
);
# Entités Latin-1, de U+00A0 à U+00FF dans l'ordre de la norme HTML : « é »
# écrit &eacute; ou en clair donne le même texte.
my @latin1 = qw(
    nbsp iexcl cent pound curren yen brvbar sect uml copy ordf laquo not shy
    reg macr deg plusmn sup2 sup3 acute micro para middot cedil sup1 ordm
    raquo frac14 frac12 frac34 iquest Agrave Aacute Acirc Atilde Auml Aring
    AElig Ccedil Egrave Eacute Ecirc Euml Igrave Iacute Icirc Iuml ETH Ntilde
    Ograve Oacute Ocirc Otilde Ouml times Oslash Ugrave Uacute Ucirc Uuml
    Yacute THORN szlig agrave aacute acirc atilde auml aring aelig ccedil
    egrave eacute ecirc euml igrave iacute icirc iuml eth ntilde ograve oacute
    ocirc otilde ouml divide oslash ugrave uacute ucirc uuml yacute thorn yuml
);
$named{ $latin1[$_] } = chr(0xA0 + $_) for 0 .. $#latin1;

sub utf8_bytes {
    my ($text) = @_;
    utf8::encode($text);
    return $text;
}

sub decode_entity {
    my ($entity) = @_;
    if ($entity =~ /^#(\d+)$/) {
        return $1 > 0x10FFFF ? "&$entity;" : utf8_bytes(chr($1));
    }
    if ($entity =~ /^#x([0-9a-f]+)$/i) {
        my $code = hex $1;
        return $code > 0x10FFFF ? "&$entity;" : utf8_bytes(chr($code));
    }
    return utf8_bytes($named{$entity}) if exists $named{$entity};
    # Entité inconnue laissée telle quelle : l'empreinte reste déterministe.
    return "&$entity;";
}

$html =~ s/&(#\d+|#x[0-9a-f]+|[a-z][a-z0-9]*);/decode_entity($1)/gie;

# Espace insécable (U+00A0) et espaces ASCII ramenées à une seule espace.
$html =~ s/\xC2\xA0/ /g;
my %fragments;
for my $fragment (split /\n/, $html) {
    $fragment =~ s/\s+/ /g;
    $fragment =~ s/^ //;
    $fragment =~ s/ $//;
    $fragments{$fragment} = 1 if length $fragment;
}

# Tri sur les octets : indépendant de la locale du serveur.
print map { "$_\n" } sort keys %fragments;
